use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use serde_json::{json, Value};
use std::{path::{Path, PathBuf}, time::{Duration, SystemTime, UNIX_EPOCH}};

pub fn now() -> u64 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64 }
fn root() -> Result<PathBuf,String> { dirs::home_dir().map(|p|p.join(".codex")).ok_or("无法确定用户目录".into()) }
fn database(root:&Path,name:&str)->Result<Connection,String>{
    let base=root.canonicalize().map_err(|_|"默认 .codex 目录不存在")?;
    let file=base.join(name).canonicalize().map_err(|_|format!("来源文件不存在：{name}"))?;
    if !file.starts_with(&base){return Err("来源文件超出默认目录".into());}
    let db=Connection::open_with_flags(file,OpenFlags::SQLITE_OPEN_READ_ONLY|OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(|_|format!("无法只读打开 {name}"))?;
    db.busy_timeout(Duration::from_secs(2)).map_err(|e|e.to_string())?;
    Ok(db)
}
pub fn valid_id(id:&str)->bool { id.len()==36&&id.chars().enumerate().all(|(i,c)|if [8,13,18,23].contains(&i){c=='-'}else{c.is_ascii_hexdigit()}) }
#[derive(Serialize)]
#[serde(rename_all="camelCase")]
pub struct Scan { pub sessions:Vec<Value>,pub observed_at:u64,pub source_path:String,pub total:u64,pub excluded:u64,pub warnings:Vec<String> }
pub fn scan()->Result<Scan,String>{scan_at(&root()?)}
fn scan_at(root:&Path)->Result<Scan,String>{
    let db=database(root,"state_5.sqlite")?;
    let excluded:u64=db.query_row("SELECT count(*) FROM threads WHERE thread_source IN ('guardian_review','subagent')",[],|r|r.get(0)).map_err(|_|"不支持的来源数据库结构")?;
    let total:u64=db.query_row("SELECT count(*) FROM threads WHERE coalesce(thread_source,'user') NOT IN ('guardian_review','subagent')",[],|r|r.get(0)).map_err(|e|e.to_string())?;
    let mut statement=db.prepare("SELECT t.id,t.title,t.cwd,t.created_at,t.updated_at,t.archived,t.model,t.project_id,p.name FROM threads t LEFT JOIN projects p ON p.id=t.project_id WHERE coalesce(t.thread_source,'user') NOT IN ('guardian_review','subagent') ORDER BY t.updated_at DESC,t.id LIMIT 2000").map_err(|_|"来源数据库结构不兼容；未修改源数据")?;
    let observed=now();
    let rows=statement.query_map([],|r|{
        let id:String=r.get(0)?;let title:String=r.get(1)?;let cwd:String=r.get(2)?;
        let created:i64=r.get(3)?;let updated:i64=r.get(4)?;let archived:bool=r.get(5)?;
        let model:Option<String>=r.get(6)?;let project_id:Option<String>=r.get(7)?;let project_name:Option<String>=r.get(8)?;
        let hub=project_id.as_ref().map(|s|format!("native:{s}")).unwrap_or("chatgpt-inbox".into());
        let name=project_name.clone().unwrap_or("ChatGPT / 待归类".into());
        Ok(json!({"id":format!("codex-local:{id}"),"tool":"chatgpt","toolName":"ChatGPT","project":hub,"projectName":name,"nativeProject":project_name.map(|n|format!("Codex 本地 / {n}")).unwrap_or_else(||if cwd.is_empty(){"Codex 本地 / 无原生项目信息".into()}else{format!("Codex 本地 / 工作目录（非原生项目）/ {cwd}")}),"title":if title.trim().is_empty(){"未命名本地会话"}else{&title},"purpose":"用途尚未整理","updated":"来源快照","messages":[],"tasks":[],"files":[],"status":"unknown","archived":archived,"source":{"kind":"codex_local","nativeId":id,"sourceTitle":title,"sourceCreatedAt":created*1000,"sourceUpdatedAt":updated*1000,"observedAt":observed,"model":model,"projectPath":cwd,"nativeProjectId":project_id,"nativeArchived":archived,"loaded":false}}))
    }).map_err(|e|e.to_string())?;
    let sessions=rows.collect::<Result<Vec<_>,_>>().map_err(|_|"来源元数据存在不可解析记录，未覆盖已有数据")?.into_iter().filter(|s|s["source"]["nativeId"].as_str().is_some_and(valid_id)).collect();
    let mut warnings=vec!["只读本机 Codex/Work 记录，不是网页 ChatGPT 全账号历史；运行状态未知。".into()];
    if total>2000{warnings.push("仅展示最近 2000 个会话；未宣称完整覆盖。".into());}
    Ok(Scan{sessions,observed_at:observed,source_path:root.to_string_lossy().into(),total,excluded,warnings})
}
pub fn read_session(id:&str)->Result<Value,String>{read_at(&root()?,id)}
fn read_at(root:&Path,id:&str)->Result<Value,String>{
    if !valid_id(id){return Err("无效的原生会话 ID".into())}
    let meta=database(root,"state_5.sqlite")?;
    let exists:bool=meta.query_row("SELECT EXISTS(SELECT 1 FROM threads WHERE id=?1 AND coalesce(thread_source,'user') NOT IN ('guardian_review','subagent'))",[id],|r|r.get(0)).map_err(|_|"无法核对来源身份")?;
    if !exists{return Err("来源会话不存在".into())}
    let db=database(root,"thread_history_1.sqlite")?;
    let mut statement=db.prepare("SELECT item_json,created_at_ms FROM thread_items WHERE thread_id=?1 AND item_type IN ('userMessage','agentMessage') ORDER BY rollout_ordinal DESC LIMIT 501").map_err(|_|"会话消息表结构不兼容")?;
    let rows=statement.query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?))).map_err(|e|e.to_string())?;
    let mut items=rows.collect::<Result<Vec<_>,_>>().map_err(|_|"来源消息记录无法解析，保留已有数据")?;let truncated=items.len()>500;
    items.truncate(500);items.reverse();
    let mut messages=Vec::new();let mut cut=false;let mut total_chars=0usize;
    for (raw,ts) in items {
        if raw.len()>2_000_000{cut=true;continue;}
        let Ok(item)=serde_json::from_str::<Value>(&raw)else{continue};
        let Some((role,text))=visible_message(&item)else{continue};
        let text=if text.chars().count()>20_000{cut=true;format!("{}\n[长消息截断，完整内容请在原工具查看]",text.chars().take(20_000).collect::<String>())}else{text};
        total_chars+=text.len();if total_chars>2_000_000{cut=true;break;}
        messages.push(json!({"role":role,"text":text,"time":"来源记录","sourceTimestamp":ts,"nativeMessageId":item["id"]}));
    }
    let assistant=messages.iter().filter(|m|m["role"]=="assistant").count();
    let mut warnings:Vec<String>=vec!["仅展示本地已落盘的用户/助手可见文本，不含私有推理和工具输出。".into()];
    if assistant==0{warnings.push("本地记录未包含助手回复，当前只显示可获取内容；不代表原会话没有回复。".into());}
    if truncated||cut{warnings.push("已触发分页或内容长度上限；当前最多展示最近 500 条消息，完整内容请打开原会话。".into());}
    Ok(json!({"messages":messages,"observedAt":now(),"warnings":warnings,"messageCount":messages.len(),"assistantCount":assistant,"truncated":truncated||cut}))
}
pub fn visible_message(item:&Value)->Option<(&'static str,String)>{
    let role=match item["type"].as_str()?{"userMessage"=>"user","agentMessage"=>"assistant",_=>return None};
    let text=if let Some(text)=item["text"].as_str(){text.to_owned()}else{item["content"].as_array()?.iter().filter_map(|c|if c["type"]=="text"{c["text"].as_str()}else{None}).collect::<Vec<_>>().join("\n")};
    let text=text.trim();
    if text.is_empty()||text.starts_with("# AGENTS.md instructions")||text.starts_with("<environment_context>")||text.starts_with("<turn_aborted>"){return None}
    let text=text.split_once("## My request:").map(|(_,request)|request.trim()).unwrap_or(text);
    if text.is_empty(){None}else{Some((role,text.to_owned()))}
}
pub fn open_web_link(value:&str)->Result<(),String>{
    let url=url::Url::parse(value).map_err(|_|"原会话链接无效")?;
    let parts=url.path().trim_matches('/').split('/').collect::<Vec<_>>();
    let valid_part=|s:&str|!s.is_empty()&&s.chars().all(|c|c.is_ascii_alphanumeric()||c=='-'||c=='_');
    let valid_path=match parts.as_slice(){["c",id]=>valid_part(id),["g",g,"c",id]=>valid_part(g)&&valid_part(id),_=>false};
    if url.scheme()!="https"||url.host_str()!=Some("chatgpt.com")||!url.username().is_empty()||url.password().is_some()||url.port().is_some()||!valid_path{return Err("仅允许 ChatGPT 原会话 HTTPS 链接".into())}
    open_url(value)
}
pub fn open_session(id:&str)->Result<(),String>{
    if !valid_id(id){return Err("无效的来源 ID".into())}
    let db=database(&root()?,"state_5.sqlite")?;
    let exists:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM threads WHERE id=?1)",[id],|r|r.get(0)).map_err(|_|"无法核对来源会话")?;
    if !exists{return Err("来源不存在".into())}
    let url=format!("codex://threads/{id}");
    open_url(&url)
}
fn open_url(url:&str)->Result<(),String>{
    #[cfg(target_os="windows")] let program="explorer.exe";
    #[cfg(target_os="macos")] let program="open";
    #[cfg(not(any(target_os="windows",target_os="macos")))] let program="xdg-open";
    std::process::Command::new(program).arg(url).spawn().map_err(|_|"无法启动协议处理程序；请确认已安装原工具")?;
    Ok(())
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn only_visible_messages_and_valid_identity(){
        assert!(valid_id("01a174b3-1a61-7830-89dd-68847d814340"));assert!(!valid_id("../../auth.json"));
        assert_eq!(visible_message(&json!({"type":"reasoning","text":"private"})),None);
        assert_eq!(visible_message(&json!({"type":"userMessage","content":[{"type":"text","text":"context\n## My request:\n整理目录"}]})).unwrap().1,"整理目录");
        assert_eq!(visible_message(&json!({"type":"agentMessage","text":"可见回复"})).unwrap().0,"assistant");
    }
}
