#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::Manager;
use std::sync::Arc;

mod server; // Reuse or link to the server logic
mod compiler; // Reuse php_embed

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Start the internal PHP server on a random port
            // Inject the port into the frontend
            let _window = app.get_window("main").unwrap();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running compiled application");
}
