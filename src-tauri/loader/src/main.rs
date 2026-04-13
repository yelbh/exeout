#![windows_subsystem = "windows"]

use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::time::{UNIX_EPOCH, Duration};
use serde::Deserialize;

use wry::{
    application::{
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    },
    webview::WebViewBuilder,
};

const SPLASH_HTML: &str = r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {
            background: linear-gradient(135deg, #0f172a 0%, #1e1b4b 100%);
            color: white;
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            display: flex; flex-direction: column; align-items: center; justify-content: center;
            height: 100vh; margin: 0; overflow: hidden;
            border: 1px solid rgba(255,255,255,0.05);
        }
        .logo { font-size: 2.5rem; font-weight: 800; margin-bottom: 2rem; color: #facc15; }
        .spinner {
            width: 40px; height: 40px;
            border: 3px solid rgba(255,255,255,0.1);
            border-left-color: #facc15;
            border-radius: 50%;
            animation: spin 1s linear infinite;
        }
        .status { margin-top: 1.5rem; font-size: 0.75rem; color: #94a3b8; letter-spacing: 0.1em; text-transform: uppercase; }
        .footer { position: absolute; bottom: 1.5rem; font-size: 0.65rem; color: #475569; }
        @keyframes spin { to { transform: rotate(360deg); } }
    </style>
</head>
<body>
    <div class="logo">ExeOutput</div>
    <div class="spinner"></div>
    <div class="status">Initialisation en cours...</div>
    <div class="footer">Conception et développement par Bensoft Services</div>
</body>
</html>
"#;

#[cfg(windows)]
fn show_error(msg: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let title: Vec<u16> = "ExeOutput Runtime Error\0".encode_utf16().collect();
    let body: Vec<u16> = format!("{}\0", msg).encode_utf16().collect();
    unsafe { MessageBoxW(0, body.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR); }
}

#[cfg(not(windows))]
fn show_error(msg: &str) {
    eprintln!("Error: {}", msg);
}

#[derive(Deserialize)]
struct Config {
    pub entry_point: String,
    pub public_dir: Option<String>,
    pub external_dirs: Option<Vec<String>>,
    pub version: Option<String>,
    pub db_type: Option<String>,
    pub db_port: Option<u32>,
    pub db_name: Option<String>,
    pub db_user: Option<String>,
    pub db_pass: Option<String>,
}

fn log(msg: &str) {
    if let Ok(mut path) = env::current_exe() {
        path.set_extension("log");
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(
                file,
                "[{}] {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                msg
            );
        }
    }
}

enum UserEvent {
    Ready(String, Vec<std::process::Child>),
    Error(String),
}

fn main() {
    let exe_path = env::current_exe().unwrap_or_default();
    let app_name = exe_path.file_stem().unwrap_or_default().to_string_lossy().to_string();
    let splash_html = SPLASH_HTML.replace("ExeOutput", &app_name);

    log(&format!("=== Démarrage {} (v1.7.7) ===", app_name));
    let event_loop = EventLoop::<UserEvent>::with_user_event();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title(&app_name)
        .with_inner_size(wry::application::dpi::LogicalSize::new(400.0, 300.0))
        .with_visible(false)
        .with_resizable(false)
        .with_decorations(false)
        .build(&event_loop)
        .unwrap();

    // Center window
    if let Some(monitor) = window.current_monitor() {
        let monitor_size = monitor.size();
        let window_size = window.outer_size();
        let x = (monitor_size.width as i32 - window_size.width as i32) / 2;
        let y = (monitor_size.height as i32 - window_size.height as i32) / 2;
        window.set_outer_position(wry::application::dpi::PhysicalPosition::new(x, y));
    }
    window.set_visible(true);

    let webview = WebViewBuilder::new(window)
        .unwrap()
        .with_html(&splash_html)
        .unwrap()
        .build()
        .unwrap();

    // Spawn extraction thread
    std::thread::spawn(move || {
        match run_background() {
            Ok((url, children)) => { let _ = proxy.send_event(UserEvent::Ready(url, children)); }
            Err(e) => { let _ = proxy.send_event(UserEvent::Error(e.to_string())); }
        }
    });

    let mut children_procs: Vec<std::process::Child> = Vec::new();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::Ready(url, children)) => {
                log("Services ready, transitioning to main window...");
                children_procs = children;
                let window = webview.window();
                window.set_title(&app_name);
                window.set_resizable(true);
                window.set_decorations(true);
                window.set_inner_size(wry::application::dpi::LogicalSize::new(1280.0, 800.0));
                
                if let Some(monitor) = window.current_monitor() {
                    let monitor_size = monitor.size();
                    let window_size = window.outer_size();
                    let x = (monitor_size.width as i32 - window_size.width as i32) / 2;
                    let y = (monitor_size.height as i32 - window_size.height as i32) / 2;
                    window.set_outer_position(wry::application::dpi::PhysicalPosition::new(x, y));
                }

                let _ = webview.load_url(&url);
                window.set_maximized(true);
            }
            Event::UserEvent(UserEvent::Error(err)) => {
                show_error(&err);
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                log("Fermeture de l'application et des services...");
                for mut child in children_procs.drain(..) {
                    let _ = child.kill();
                }
                *control_flow = ControlFlow::Exit;
            }
            _ => (),
        }
    });
}

fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(&entry.path(), &dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

fn run_background() -> Result<(String, Vec<std::process::Child>), Box<dyn std::error::Error + Send + Sync>> {
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or("Cannot find EXE directory")?;
    let exe_file = File::open(&exe_path)?;
    let exe_metadata = fs::metadata(&exe_path)?;
    let modified = exe_metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
    let size = exe_metadata.len();
    let file_name = exe_path.file_name().unwrap_or_default().to_string_lossy().into_owned();

    let mut children = Vec::new();

    // ── 1. Extraction ──────────────────────────────────────────────────────────
    let temp_dir = env::temp_dir().join(format!("exeoutput_cache_{}_{}_{}", file_name, modified, size));
    fs::create_dir_all(&temp_dir)?;

    let extraction_marker = temp_dir.join(".extraction_ok");
    if !extraction_marker.exists() {
        log("Démarrage de l'extraction...");
        let mut archive = zip::ZipArchive::new(exe_file)?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = match file.enclosed_name() {
                Some(path) => temp_dir.join(path),
                None => continue,
            };
            if file.name().ends_with('/') {
                let _ = fs::create_dir_all(&outpath);
            } else {
                if let Some(p) = outpath.parent() { let _ = fs::create_dir_all(p); }
                if let Ok(mut outfile) = fs::File::create(&outpath) { let _ = io::copy(&mut file, &mut outfile); }
            }
        }
        let _ = fs::write(&extraction_marker, "ok");
        log("Extraction terminée.");
    }

    // ── 2. Config & External Mappings ──────────────────────────────────────────
    let config_path = temp_dir.join("exeoutput.json");
    let config_file = File::open(&config_path)?;
    let config: Config = serde_json::from_reader(config_file)?;

    let data_dir = exe_dir.join("data");
    fs::create_dir_all(&data_dir)?;
    
    let mut external_dirs = config.external_dirs.clone().unwrap_or_default();
    external_dirs.retain(|d| d != "bootstrap");
    if !external_dirs.contains(&"bootstrap/cache".to_string()) {
        external_dirs.push("bootstrap/cache".to_string());
    }

    log("Initialisation des dossiers externes...");
    for dir in &external_dirs {
        let src_in_data = data_dir.join(dir);
        let dst_in_temp = temp_dir.join(dir);
        if !src_in_data.exists() && dst_in_temp.is_dir() {
            log(&format!("Premier lancement : initialisation de data/{}...", dir));
            let _ = copy_dir_recursive(&dst_in_temp, &src_in_data);
        }
        if src_in_data.is_dir() {
            if let Some(p) = dst_in_temp.parent() { let _ = fs::create_dir_all(p); }
            if dst_in_temp.exists() {
                let _ = Command::new("cmd").args(&["/c", "rmdir", "/s", "/q", dst_in_temp.to_str().unwrap()]).creation_flags(0x08000000).status();
            }
            let _ = Command::new("cmd").args(&["/c", "mklink", "/j", dst_in_temp.to_str().unwrap(), src_in_data.to_str().unwrap()]).creation_flags(0x08000000).status();
        }
    }

    // ── 2b. Multiposte support: External .env override ────────────────────────
    let external_env = exe_dir.join(".env");
    if external_env.exists() {
        log("Dispositif Multiposte : Fichier .env externe détecté. Fusion avec la configuration interne...");
        let target_env = temp_dir.join(".env");
        
        let mut internal_lines: Vec<String> = match fs::read_to_string(&target_env) {
            Ok(content) => content.lines().map(|s| s.to_string()).collect(),
            Err(_) => Vec::new(),
        };

        if let Ok(external_content) = fs::read_to_string(&external_env) {
            for ext_line in external_content.lines() {
                let ext_line_trim = ext_line.trim();
                
                // Ignore empty lines
                if ext_line_trim.is_empty() {
                    continue;
                }
                
                // If it's a comment, just append it
                if ext_line_trim.starts_with('#') {
                    internal_lines.push(ext_line.to_string());
                    continue;
                }
                
                // If it has a key=value format, remove the existing key and append the new one
                if let Some(eq_idx) = ext_line.find('=') {
                    let key = ext_line[..eq_idx].trim();
                    internal_lines.retain(|line| {
                        let line_trim = line.trim_start();
                        !line_trim.starts_with(&(key.to_string() + "="))
                    });
                }
                internal_lines.push(ext_line.to_string());
            }
        }
        
        let _ = fs::write(&target_env, internal_lines.join("\n"));
    }

    // ── 3. Internal Back-Bridge ───────────────────────────────────────────────
    if let Ok(entries) = fs::read_dir(&temp_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap().to_str().unwrap();
                    if !external_dirs.contains(&name.to_string()) && !data_dir.join(name).exists() {
                        let bridge_src = data_dir.join(name);
                        let _ = Command::new("cmd").args(&["/c", "mklink", "/j", bridge_src.to_str().unwrap(), path.to_str().unwrap()]).creation_flags(0x08000000).status();
                        // Hide the junction to keep data/ folder clean
                        let _ = Command::new("attrib").args(&["+h", bridge_src.to_str().unwrap()]).creation_flags(0x08000000).status();
                    }
                }
            }
        }
    }

    // ── 4. MariaDB Startup ────────────────────────────────────────────────────
    let mut db_ready = true;
    if let Some(db_type) = &config.db_type {
        if db_type == "mariadb" {
            db_ready = false;
            let mysql_dir = data_dir.join("mysql");
            let mysqld_exe = mysql_dir.join("bin").join("mysqld.exe");
            if mysqld_exe.exists() {
                let db_data_dir = mysql_dir.join("data");
                let db_port = config.db_port.unwrap_or(3307);
                log(&format!("Démarrage de MariaDB sur le port {}...", db_port));
                
                let mut db_cmd = Command::new(&mysqld_exe);
                db_cmd.arg("--no-defaults")
                      .arg(format!("--datadir={}", db_data_dir.to_str().unwrap()))
                      .arg(format!("--port={}", db_port))
                      .arg("--bind-address=127.0.0.1")
                      .arg("--skip-grant-tables")
                      .arg("--console")
                      .creation_flags(0x08000000);

                match db_cmd.spawn() {
                    Ok(child) => {
                        children.push(child);
                        for _ in 0..100 {
                            if std::net::TcpStream::connect(("127.0.0.1", db_port as u16)).is_ok() {
                                db_ready = true;
                                log("MariaDB est prête.");
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(100));
                        }
                    }
                    Err(e) => log(&format!("Erreur lancement MariaDB : {}", e)),
                }
            }
        }
    }

    // ── 5. Smart Root Detection ───────────────────────────────────────────────
    let mut selected_doc_root = temp_dir.clone();
    if let Some(public) = &config.public_dir {
        let candidate = temp_dir.join(public);
        if candidate.is_dir() { selected_doc_root = candidate; }
    }
    if !selected_doc_root.join("index.php").exists() {
        let auto_public = temp_dir.join("public");
        if auto_public.is_dir() && auto_public.join("index.php").exists() { selected_doc_root = auto_public; }
    }

    // ── 6. PHP Server ──────────────────────────────────────────────────────────
    let cache_dir = temp_dir.join("bootstrap").join("cache");
    if cache_dir.exists() {
        if let Ok(entries) = fs::read_dir(&cache_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_file() && path.file_name().unwrap() != ".gitignore" {
                        let _ = fs::remove_file(path);
                    }
                }
            }
        }
    }

    log("Démarrage du serveur PHP...");
    let mut php_cmd = Command::new("php");
    php_cmd.arg("-S").arg("127.0.0.1:8080").arg("-t").arg(&selected_doc_root).current_dir(&temp_dir);
    
    if let Some(db_type) = &config.db_type {
        if db_type != "none" {
            php_cmd.env("DB_CONNECTION", if db_type == "mariadb" { "mysql" } else { db_type });
            php_cmd.env("DB_HOST", "127.0.0.1");
            php_cmd.env("DB_PORT", config.db_port.unwrap_or(3307).to_string());
            php_cmd.env("DB_DATABASE", config.db_name.as_deref().unwrap_or(""));
            php_cmd.env("DB_USERNAME", config.db_user.as_deref().unwrap_or("root"));
            php_cmd.env("DB_PASSWORD", config.db_pass.as_deref().unwrap_or(""));
        }
    }
    
    let server_php = temp_dir.join("server.php");
    if server_php.exists() { php_cmd.arg(&server_php); }
    
    php_cmd.creation_flags(0x08000000); 
    let child = php_cmd.spawn()?;
    children.push(child);

    // Wait for PHP and Execute SQL logic
    for _ in 0..50 {
        if std::net::TcpStream::connect("127.0.0.1:8080").is_ok() { 
            log("Le serveur PHP est prêt.");
            
            // ── 7. SQL Import Logic ───────────────────────────────────────────────
            if db_ready {
                let db_port = config.db_port.unwrap_or(3307);
                let db_name = config.db_name.as_deref().unwrap_or("");
                let mysql_exe = data_dir.join("mysql").join("bin").join("mysql.exe");

                if mysql_exe.exists() {
                    // Check for internal init.sql
                    let init_sql = data_dir.join("init.sql");
                    if init_sql.exists() {
                        log("Initialisation : Fichier init.sql détecté.");
                        let cmd_str = format!(
                            "type \"{}\" | \"{}\" -u root -P{} {}",
                            init_sql.to_str().unwrap(),
                            mysql_exe.to_str().unwrap(),
                            db_port, db_name
                        );
                        let _ = Command::new("cmd").args(&["/c", &cmd_str]).creation_flags(0x08000000).status();
                        let _ = fs::remove_file(init_sql);
                    }

                    // Check for external import.sql
                    let import_sql = exe_dir.join("import.sql");
                    if import_sql.exists() {
                        log("Mise à jour : Fichier import.sql détecté.");
                        let cmd_str = format!(
                            "type \"{}\" | \"{}\" -u root -P{} {}",
                            import_sql.to_str().unwrap(),
                            mysql_exe.to_str().unwrap(),
                            db_port, db_name
                        );
                        if Command::new("cmd").args(&["/c", &cmd_str]).creation_flags(0x08000000).status().is_ok() {
                            let _ = fs::rename(&import_sql, exe_dir.join("import.sql.done"));
                        }
                    }

                    // ── 7b. Cascade: Initial Pull from central server ─────────────
                    // Only if: no import.sql was found AND no .sync_pull_done marker exists
                    let pull_done_marker = data_dir.join(".sync_pull_done");
                    let import_sql_done = exe_dir.join("import.sql.done");
                    let import_sql_exists = exe_dir.join("import.sql").exists();
                    
                    if !pull_done_marker.exists() && !import_sql_exists && !import_sql_done.exists() {
                        // Check if ENTITE_ID is configured
                        let env_path = temp_dir.join(".env");
                        let mut entite_id_ok = false;
                        if let Ok(content) = fs::read_to_string(&env_path) {
                            for line in content.lines() {
                                if line.starts_with("ENTITE_ID=") {
                                    let val = line.replace("ENTITE_ID=", "").trim().to_string();
                                    if !val.is_empty() && val != "CHANGE_ME" && val != "12345" {
                                        entite_id_ok = true;
                                    }
                                }
                            }
                        }

                        if entite_id_ok {
                            log("Premier lancement : tentative de récupération des données depuis la plateforme centrale...");
                            let pull_status = Command::new("php")
                                .arg("artisan").arg("parois:sync-pull")
                                .current_dir(&temp_dir)
                                .creation_flags(0x08000000)
                                .status();
                            match pull_status {
                                Ok(s) if s.success() => {
                                    log("Données initiales récupérées avec succès depuis la plateforme centrale.");
                                    let _ = fs::write(&pull_done_marker, "ok");
                                }
                                Ok(_) => log("Le pull initial a échoué (réponse non-succès). La base de données locale sera utilisée telle quelle."),
                                Err(e) => log(&format!("Erreur lors du pull initial : {}. La base locale sera utilisée.", e)),
                            }
                        } else {
                            log("Premier lancement : ENTITE_ID non configuré, pull initial ignoré. Placez un import.sql à côté de l'EXE ou configurez ENTITE_ID dans .env.");
                        }
                    }
                }
            }

            // ── 8. Auto-Sync Background Task ──────────────────────────────────────────
            let sync_temp_dir = temp_dir.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(60)); // Startup Catch-up
                loop {
                    let mut enabled = true;
                    let env_path = sync_temp_dir.join(".env");
                    if let Ok(content) = fs::read_to_string(&env_path) {
                        if content.contains("AUTO_SYNC=false") { enabled = false; }
                    }

                    if enabled {
                        // Security check: Verify ENTITE_ID
                        let mut entite_id_ok = false;
                        if let Ok(content) = fs::read_to_string(&env_path) {
                            for line in content.lines() {
                                if line.starts_with("ENTITE_ID=") {
                                    let val = line.replace("ENTITE_ID=", "").trim().to_string();
                                    if !val.is_empty() && val != "CHANGE_ME" && val != "12345" {
                                        entite_id_ok = true;
                                    }
                                }
                            }
                        }

                        if entite_id_ok {
                            log("Synchronisation automatique en cours (parois:sync-push)...");
                            let _ = Command::new("php")
                                .arg("artisan").arg("parois:sync-push")
                                .current_dir(&sync_temp_dir)
                                .creation_flags(0x08000000)
                                .status();
                        } else {
                            log("Synchronisation automatique ignorée : ENTITE_ID non configuré ou invalide dans le .env");
                        }
                    }
                    std::thread::sleep(Duration::from_secs(900)); // 15 minutes
                }
            });

            return Ok(("http://127.0.0.1:8080".to_string(), children)); 
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    
    Ok(("http://127.0.0.1:8080".to_string(), children))
}
