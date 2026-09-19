// 隐藏发布版的控制台窗口（Windows）
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    gemini_draw_lib::run()
}
