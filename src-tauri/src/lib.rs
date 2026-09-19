mod commands;
mod engine;
mod model;
mod paths;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let root = paths::data_dir().expect("无法确定应用数据目录");
            paths::ensure_layout(&root).expect("无法创建应用数据目录");
            app.manage(commands::AppState { root });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_snapshot,
            commands::save_config,
            commands::install_cli,
            commands::import_cookies,
            commands::check_account,
            commands::load_tasks,
            commands::save_tasks,
            commands::import_ref_file,
            commands::import_ref_base64,
            commands::remove_ref,
            commands::generate_task,
            commands::read_image,
            commands::open_path,
            commands::pick_directory,
            commands::save_image_as,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Gemini Draw 失败");
}
