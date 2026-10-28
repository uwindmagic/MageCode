#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod source_bridge;

#[tauri::command]
async fn scan_default_source()->Result<magecode_local_source::Scan,String>{
    tauri::async_runtime::spawn_blocking(magecode_local_source::scan).await.map_err(|_|"来源读取任务失败")?
}
#[tauri::command]
async fn read_source_session(id:String)->Result<serde_json::Value,String>{
    tauri::async_runtime::spawn_blocking(move||magecode_local_source::read_session(&id)).await.map_err(|_|"来源读取任务失败")?
}
#[tauri::command]
fn open_source_session(id:String)->Result<(),String>{magecode_local_source::open_session(&id)}
#[tauri::command]
fn open_chatgpt_link(url:String)->Result<(),String>{magecode_local_source::open_web_link(&url)}

fn main() {
    if std::env::args().any(|s|s=="--source-server"){
        if let Err(e)=source_bridge::run(){eprintln!("来源桥接启动失败：{e}");std::process::exit(1)}
        return;
    }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![scan_default_source,read_source_session,open_source_session,open_chatgpt_link])
        .run(tauri::generate_context!())
        .expect("无法启动 MageCode 桌面窗口");
}
