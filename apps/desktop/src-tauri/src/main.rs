#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod codex;
mod store;

#[tauri::command]
async fn list_codex_sessions() -> Result<codex::CodexLibrary, String> {
    tauri::async_runtime::spawn_blocking(|| store::Store::open_default()?.codex_library())
        .await
        .map_err(|error| format!("读取本地会话库时任务中断：{error}"))?
}

#[tauri::command]
async fn sync_codex() -> Result<codex::CodexLibrary, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let snapshot = codex::collect_snapshot()?;
        let store = store::Store::open_default()?;
        store.sync_codex(snapshot)?;
        store.codex_library()
    })
        .await
        .map_err(|error| format!("同步 Codex 会话时任务中断：{error}"))?
}

#[tauri::command]
async fn read_codex_messages(native_id: String) -> Result<Vec<codex::CodexMessage>, String> {
    tauri::async_runtime::spawn_blocking(move || store::Store::open_default()?.codex_messages(&native_id))
        .await
        .map_err(|error| format!("读取本地消息时任务中断：{error}"))?
}

#[tauri::command]
fn open_codex_session(native_id: String) -> Result<(), String> {
    codex::open_native(&native_id)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            list_codex_sessions,
            sync_codex,
            read_codex_messages,
            open_codex_session
        ])
        .run(tauri::generate_context!())
        .expect("无法启动 MageCode 桌面窗口");
}
