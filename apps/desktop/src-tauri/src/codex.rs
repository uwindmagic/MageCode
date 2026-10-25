use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

const SOURCE_KIND: &str = "codex-local";
const MAX_MESSAGE_CHARS: usize = 100_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexMessage {
    pub native_id: String,
    pub role: String,
    pub text: String,
    pub time: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSession {
    pub id: String,
    pub native_id: String,
    pub tool: String,
    pub tool_name: String,
    pub project: String,
    pub project_name: String,
    pub native_project: String,
    pub title: String,
    pub purpose: String,
    pub updated: String,
    pub last_active_at: Option<String>,
    pub archived: bool,
    pub source: String,
    pub source_url: String,
    pub messages: Vec<CodexMessage>,
    pub tasks: Vec<serde_json::Value>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSource {
    pub root: String,
    pub loaded_at: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexLibrary {
    pub source: CodexSource,
    pub sessions: Vec<CodexSession>,
}

#[derive(Debug, Deserialize)]
struct SessionIndexEntry {
    id: String,
    thread_name: Option<String>,
    updated_at: Option<String>,
}

#[derive(Debug, Clone)]
struct SourceFile {
    path: PathBuf,
    native_id: String,
    cwd: String,
    archived: bool,
    modified: SystemTime,
}

pub fn default_root() -> Result<PathBuf, String> {
    if let Some(value) = std::env::var_os("CODEX_HOME") {
        if !value.is_empty() {
            return Ok(PathBuf::from(value));
        }
    }
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| "无法确定用户目录，请设置 CODEX_HOME 后重试。".to_string())?;
    Ok(PathBuf::from(home).join(".codex"))
}

#[cfg(test)]
pub fn load_library() -> Result<CodexLibrary, String> {
    scan_library(false)
}

pub fn collect_snapshot() -> Result<CodexLibrary, String> {
    scan_library(true)
}

fn scan_library(include_messages: bool) -> Result<CodexLibrary, String> {
    let root = default_root()?;
    if !root.is_dir() {
        return Err(format!("未找到 Codex 数据目录：{}", root.display()));
    }
    let index = read_index(&root.join("session_index.jsonl"));
    let mut files = discover_files(&root)?;
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    let mut seen = HashSet::new();
    let mut sessions = Vec::new();
    for file in files {
        if !seen.insert(file.native_id.clone()) {
            continue;
        }
        let entry = index.get(&file.native_id);
        let title = entry
            .and_then(|item| item.thread_name.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("Codex 会话 {}", &file.native_id[..8]));
        let project_name = Path::new(&file.cwd)
            .file_name()
            .and_then(|value| value.to_str())
            .filter(|value| !value.is_empty())
            .unwrap_or("未指定项目")
            .to_string();
        let last_active_at = entry.and_then(|item| item.updated_at.clone());
        let messages = if include_messages { parse_messages(&file.path)? } else { Vec::new() };
        sessions.push(CodexSession {
            id: format!("codex:{}", file.native_id),
            source_url: format!("codex://threads/{}", file.native_id),
            native_id: file.native_id,
            tool: "chatgpt".to_string(),
            tool_name: "ChatGPT".to_string(),
            project: project_id(&file.cwd),
            project_name,
            native_project: file.cwd,
            title,
            purpose: "本机 Codex 只读会话；标题与项目来自原生记录。".to_string(),
            updated: last_active_at.clone().unwrap_or_else(|| "未知".to_string()),
            last_active_at,
            archived: file.archived,
            source: SOURCE_KIND.to_string(),
            messages,
            tasks: Vec::new(),
            files: Vec::new(),
        });
    }
    sessions.sort_by(|a, b| b.last_active_at.cmp(&a.last_active_at));
    Ok(CodexLibrary {
        source: CodexSource {
            root: root.display().to_string(),
            loaded_at: now_timestamp(),
            read_only: true,
        },
        sessions,
    })
}

#[cfg(test)]
pub fn read_messages(native_id: &str) -> Result<Vec<CodexMessage>, String> {
    validate_native_id(native_id)?;
    let root = default_root()?;
    let mut files = discover_files(&root)?;
    files.sort_by(|a, b| b.modified.cmp(&a.modified));
    let file = files
        .into_iter()
        .find(|item| item.native_id == native_id)
        .ok_or_else(|| "未找到对应的 Codex 会话，可能已移动或删除。".to_string())?;
    parse_messages(&file.path)
}

pub fn open_native(native_id: &str) -> Result<(), String> {
    validate_native_id(native_id)?;
    let root = default_root()?;
    if !discover_files(&root)?
        .into_iter()
        .any(|item| item.native_id == native_id)
    {
        return Err("未找到对应的 Codex 会话，未执行跳转。".to_string());
    }
    let url = format!("codex://threads/{native_id}");
    #[cfg(target_os = "windows")]
    let result = Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", &url])
        .spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(&url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(&url).spawn();
    result
        .map(|_| ())
        .map_err(|error| format!("无法打开 Codex 原会话：{error}"))
}

fn read_index(path: &Path) -> HashMap<String, SessionIndexEntry> {
    let mut entries = HashMap::new();
    let Ok(file) = File::open(path) else {
        return entries;
    };
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if let Ok(entry) = serde_json::from_str::<SessionIndexEntry>(&line) {
            entries.insert(entry.id.clone(), entry);
        }
    }
    entries
}

fn discover_files(root: &Path) -> Result<Vec<SourceFile>, String> {
    let mut files = Vec::new();
    for (directory, archived) in [("sessions", false), ("archived_sessions", true)] {
        let path = root.join(directory);
        if path.is_dir() {
            collect_jsonl(&path, archived, &mut files)?;
        }
    }
    Ok(files)
}

fn collect_jsonl(directory: &Path, archived: bool, output: &mut Vec<SourceFile>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("无法读取 Codex 目录 {}：{error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("无法读取 Codex 目录项：{error}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl(&path, archived, output)?;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        if let Some(file) = read_source_file(&path, archived) {
            output.push(file);
        }
    }
    Ok(())
}

fn read_source_file(path: &Path, archived: bool) -> Option<SourceFile> {
    let file = File::open(path).ok()?;
    let mut first_line = String::new();
    BufReader::new(file).read_line(&mut first_line).ok()?;
    let record: serde_json::Value = serde_json::from_str(&first_line).ok()?;
    if record.get("type")?.as_str()? != "session_meta" {
        return None;
    }
    let payload = record.get("payload")?;
    if payload.get("thread_source").and_then(|value| value.as_str()) != Some("user") {
        return None;
    }
    let native_id = payload.get("id")?.as_str()?.to_string();
    if validate_native_id(&native_id).is_err() {
        return None;
    }
    let cwd = payload
        .get("cwd")
        .and_then(|value| value.as_str())
        .unwrap_or("未指定项目")
        .to_string();
    let modified = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .unwrap_or(UNIX_EPOCH);
    Some(SourceFile {
        path: path.to_path_buf(),
        native_id,
        cwd,
        archived,
        modified,
    })
}

fn parse_messages(path: &Path) -> Result<Vec<CodexMessage>, String> {
    let file = File::open(path)
        .map_err(|error| format!("无法打开 Codex 会话 {}：{error}", path.display()))?;
    let mut messages = Vec::new();
    let mut seen = HashSet::new();
    for line in BufReader::new(file).lines() {
        let line = line.map_err(|error| format!("读取 Codex 会话失败：{error}"))?;
        if !line.contains("\"type\":\"message\"") {
            continue;
        }
        let Ok(record) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if record.get("type").and_then(|value| value.as_str()) != Some("response_item") {
            continue;
        }
        let Some(payload) = record.get("payload") else {
            continue;
        };
        if payload.get("type").and_then(|value| value.as_str()) != Some("message") {
            continue;
        }
        let Some(id) = payload.get("id").and_then(|value| value.as_str()) else {
            continue;
        };
        let Some(role) = payload.get("role").and_then(|value| value.as_str()) else {
            continue;
        };
        if role != "user" && role != "assistant" {
            continue;
        }
        // Codex assistant commentary is process output, not a conversation result.
        // Missing/unknown phases are deliberately excluded rather than guessed to be final.
        if role == "assistant" && payload.get("phase").and_then(|value| value.as_str()) != Some("final") {
            continue;
        }
        if !seen.insert(id.to_string()) {
            continue;
        }
        let mut parts = Vec::new();
        for content in payload
            .get("content")
            .and_then(|value| value.as_array())
            .into_iter()
            .flatten()
        {
            let kind = content.get("type").and_then(|value| value.as_str());
            if kind != Some("input_text") && kind != Some("output_text") {
                continue;
            }
            if let Some(text) = content.get("text").and_then(|value| value.as_str()) {
                let cleaned = clean_visible_text(text);
                if !cleaned.is_empty() {
                    parts.push(cleaned);
                }
            }
        }
        if parts.is_empty() {
            continue;
        }
        let mut text = parts.join("\n\n");
        if text.chars().count() > MAX_MESSAGE_CHARS {
            text = text.chars().take(MAX_MESSAGE_CHARS).collect();
            text.push_str("\n\n[消息过长，已在界面中截断；原始记录未被修改。]");
        }
        messages.push(CodexMessage {
            native_id: id.to_string(),
            role: role.to_string(),
            text,
            time: record
                .get("timestamp")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string(),
        });
    }
    Ok(messages)
}

fn clean_visible_text(text: &str) -> String {
    let mut cleaned = text.to_string();
    for tag in [
        "environment_context",
        "permissions instructions",
        "in-app-browser-context",
        "external_codex_apps_open_page",
        "turn_aborted",
    ] {
        cleaned = remove_xml_block(&cleaned, tag);
    }
    cleaned
        .replace("## My request:", "")
        .trim()
        .to_string()
}

fn remove_xml_block(text: &str, tag: &str) -> String {
    let opening = format!("<{tag}");
    let closing = format!("</{tag}>");
    let mut output = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(&opening) {
        output.push_str(&rest[..start]);
        let after_tag = &rest[start + opening.len()..];
        let Some(opening_end) = after_tag.find('>') else {
            rest = "";
            break;
        };
        let after_open = &after_tag[opening_end + 1..];
        if let Some(end) = after_open.find(&closing) {
            rest = &after_open[end + closing.len()..];
        } else {
            rest = "";
            break;
        }
    }
    output.push_str(rest);
    output
}

pub fn validate_native_id(value: &str) -> Result<(), String> {
    let valid_length = value.len() == 36;
    let valid_characters = value
        .chars()
        .all(|character| character.is_ascii_hexdigit() || character == '-');
    if valid_length && valid_characters {
        Ok(())
    } else {
        Err("无效的 Codex 会话 ID。".to_string())
    }
}

fn project_id(cwd: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in cwd.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("codex-project-{hash:016x}")
}

fn now_timestamp() -> String {
    // The UI treats this as a freshness label; message and session times retain source RFC3339 values.
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => "0".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_automatic_context_without_removing_request() {
        let input = "<in-app-browser-context source=\"ambient-ui-state\">state</in-app-browser-context>\n## My request:\n同步会话";
        assert_eq!(clean_visible_text(input), "同步会话");
    }

    #[test]
    fn validates_native_ids_without_accepting_paths_or_urls() {
        assert!(validate_native_id("01a17456-ac5d-7592-a93e-e8430a661fea").is_ok());
        assert!(validate_native_id("../../etc/passwd").is_err());
        assert!(validate_native_id("codex://threads/test").is_err());
    }

    #[test]
    fn project_ids_are_stable_and_do_not_expose_paths() {
        let id = project_id("H:\\PROJECTS\\PARA\\AI_MP_AgentTasks");
        assert_eq!(id, project_id("H:\\PROJECTS\\PARA\\AI_MP_AgentTasks"));
        assert!(!id.contains("PROJECTS"));
        assert_ne!(id, project_id("H:\\another-project"));
    }

    #[test]
    fn parses_only_visible_user_and_assistant_messages() {
        let root = std::env::temp_dir().join(format!("magecode-codex-test-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("session.jsonl");
        fs::write(
            &path,
            concat!(
                "{\"type\":\"response_item\",\"timestamp\":\"2026-10-25T10:00:00Z\",\"payload\":{\"type\":\"message\",\"id\":\"m1\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"真实需求\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"id\":\"m2\",\"role\":\"developer\",\"content\":[{\"type\":\"input_text\",\"text\":\"隐藏指令\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"id\":\"m3\",\"role\":\"assistant\",\"phase\":\"commentary\",\"content\":[{\"type\":\"output_text\",\"text\":\"过程评论\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"id\":\"m4\",\"role\":\"assistant\",\"content\":[{\"type\":\"output_text\",\"text\":\"无法确认的旧记录\"}]}}\n",
                "{\"type\":\"response_item\",\"payload\":{\"type\":\"function_call\",\"id\":\"m5\",\"name\":\"shell\"}}\n",
                "{\"type\":\"response_item\",\"timestamp\":\"2026-10-25T10:01:00Z\",\"payload\":{\"type\":\"message\",\"id\":\"m6\",\"role\":\"assistant\",\"phase\":\"final\",\"content\":[{\"type\":\"output_text\",\"text\":\"已完成\"}]}}\n"
            ),
        )
        .unwrap();
        let messages = parse_messages(&path).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].text, "真实需求");
        assert_eq!(messages[1].text, "已完成");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    #[ignore = "reads the current user's private Codex history; run explicitly for local smoke verification"]
    fn reads_default_source_without_exposing_message_content() {
        let library = load_library().expect("default Codex source should be readable");
        assert!(!library.sessions.is_empty());
        assert!(library.sessions.iter().all(|session| {
            session.source == SOURCE_KIND
                && session.messages.is_empty()
                && validate_native_id(&session.native_id).is_ok()
        }));
        let readable = library.sessions.iter().any(|session| {
            read_messages(&session.native_id)
                .map(|messages| !messages.is_empty())
                .unwrap_or(false)
        });
        assert!(readable, "at least one session should contain visible messages");
    }
}
