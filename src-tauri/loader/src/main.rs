#![windows_subsystem = "windows"]

use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::process::{Command};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::time::{UNIX_EPOCH, Duration};
use std::sync::{Arc};
use serde::Deserialize;

// Windows API imports
use windows::{
    core::*,
    Win32::Foundation::*,
    Win32::System::LibraryLoader::GetModuleHandleW,
    Win32::UI::WindowsAndMessaging::*,
    Win32::System::Com::*,
};
use webview2_com::*;

#[derive(Deserialize)]
struct Config {
    pub entry_point: String,
    pub public_dir: Option<String>,
    pub external_dirs: Option<Vec<String>>,
    pub version: Option<String>,
}

fn log(msg: &str) {
    if let Ok(mut path) = env::current_exe() {
        path.set_extension("log");
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), msg);
        }
    }
}

fn main() {
    log("=== Lancement de l'application (v1.6.5 Smart Window) ===");
    if let Err(e) = run() {
        let msg = format!("Erreur fatale : {}", e);
        log(&msg);
        let _ = rfd::MessageDialog::new()
            .set_title("ExeOutput Runtime Error")
            .set_description(&msg)
            .set_level(rfd::MessageLevel::Error)
            .show();
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = env::current_exe()?;
    let exe_file = File::open(&exe_path)?;
    
    let exe_metadata = fs::metadata(&exe_path)?;
    let modified = exe_metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
    let size = exe_metadata.len();
    let file_name = exe_path.file_name().unwrap_or_default().to_string_lossy().into_owned();

    // 1. Extraction
    let mut archive = zip::ZipArchive::new(exe_file)?;
    let temp_dir = env::temp_dir().join(format!("exeoutput_cache_{}_{}_{}", file_name, modified, size));
    fs::create_dir_all(&temp_dir)?;
    
    let extraction_marker = temp_dir.join(".extraction_ok");
    if !extraction_marker.exists() {
        let total_files = archive.len();
        for i in 0..total_files {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => temp_dir.join(path),
                None => continue,
            };
            if (*file.name()).ends_with('/') {
                let _ = fs::create_dir_all(&outpath);
            } else {
                if let Some(p) = outpath.parent() { let _ = fs::create_dir_all(p); }
                if let Ok(mut outfile) = fs::File::create(&outpath) {
                    let _ = io::copy(&mut file, &mut outfile);
                }
            }
        }
        let _ = fs::write(&extraction_marker, "ok");
    }

    // 2. Config
    let config_path = temp_dir.join("exeoutput.json");
    let config_file = File::open(config_path)?;
    let config: Config = serde_json::from_reader(config_file)?;

    // 3. Mapping (Simplified for brevity, following previous logic)
    let exe_dir = exe_path.parent().unwrap();
    let data_dir = exe_dir.join("data");
    
    let mut external_dirs = config.external_dirs.clone().unwrap_or_default();
    external_dirs.retain(|d| d != "bootstrap"); // Force internal bootstrap
    if !external_dirs.contains(&"bootstrap/cache".to_string()) {
        external_dirs.push("bootstrap/cache".to_string());
    }

    for dir in &external_dirs {
        let src = data_dir.join(dir);
        let dst = temp_dir.join(dir);
        if src.is_dir() {
            if let Some(p) = dst.parent() { let _ = fs::create_dir_all(p); }
            if dst.exists() {
                let _ = Command::new("cmd").args(&["/c", "rmdir", "/s", "/q", dst.to_str().unwrap()]).creation_flags(0x08000000).status();
            }
            let _ = Command::new("cmd").args(&["/c", "mklink", "/j", dst.to_str().unwrap(), src.to_str().unwrap()]).creation_flags(0x08000000).status();
        }
    }

    // 4. Smart Routing
    let mut selected_doc_root = temp_dir.clone();
    if let Some(public) = &config.public_dir {
        if !public.is_empty() { selected_doc_root = temp_dir.join(public); }
    }
    if !selected_doc_root.join("index.php").exists() {
        let auto_public = temp_dir.join("public");
        if auto_public.is_dir() && auto_public.join("index.php").exists() {
            selected_doc_root = auto_public;
            log("Auto-détection du dossier public.");
        }
    }

    // 5. Lancement PHP (sur place pour éviter le 404)
    log(&format!("Lancement PHP dans {}", selected_doc_root.display()));
    let mut php_process = Command::new("php");
    php_process.arg("-S").arg("127.0.0.1:8080")
               .current_dir(&selected_doc_root)
               .creation_flags(0x08000000); 
    let mut child = php_process.spawn()?;

    // 6. Fenêtre Autonome (Win32 + WebView2)
    log("Démarrage de la fenêtre autonome...");
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        
        let h_instance = GetModuleHandleW(None)?;
        let window_class = "ExeOutputWindow";
        let mut wc = WNDCLASSW::default();
        wc.lpfnWndProc = Some(wnd_proc);
        wc.hInstance = h_instance;
        wc.lpszClassName = PCWSTR(format!("{}\0", window_class).encode_utf16().collect::<Vec<u16>>().as_ptr());
        
        RegisterClassW(&wc);
        
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            wc.lpszClassName,
            PCWSTR("Application Autonome\0".encode_utf16().collect::<Vec<u16>>().as_ptr()),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT, CW_USEDEFAULT, 1280, 800,
            None, None, h_instance, None,
        );

        let _ = create_webview(hwnd, "http://127.0.0.1:8080");

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    let _ = child.kill();
    Ok(())
}

fn create_webview(hwnd: HWND, url: &str) -> i32 {
    use webview2_com::Microsoft::Web::WebView2::Win32::*;
    let url_string = url.to_string();
    
    unsafe {
        let _ = CreateCoreWebView2EnvironmentCompletedHandler::create(move |_result, env| {
            let _ = env.unwrap().CreateCoreWebView2Controller(hwnd, &CreateCoreWebView2ControllerCompletedHandler::create(move |_result, controller| {
                if let Some(controller) = controller {
                    let mut rect = RECT::default();
                    let _ = GetClientRect(hwnd, &mut rect);
                    let _ = controller.SetBounds(rect);
                    let _ = controller.SetIsVisible(true);
                    
                    let webview = controller.get_CoreWebView2().unwrap();
                    let _ = webview.Navigate(PCWSTR(format!("{}\0", url_string).encode_utf16().collect::<Vec<u16>>().as_ptr()));
                }
                Ok(())
            }));
            Ok(())
        });
        0
    }
}

extern "system" fn wnd_proc(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_SIZE => {
                // Should resize webview here, but for simplicity we keep it as is
                DefWindowProcW(window, message, wparam, lparam)
            }
            _ => DefWindowProcW(window, message, wparam, lparam),
        }
    }
}
