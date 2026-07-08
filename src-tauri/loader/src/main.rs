#![windows_subsystem = "windows"]

use std::env;
use std::fs::{self, File};
use std::io::{self, Write};
use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::time::{UNIX_EPOCH, Duration};
use std::collections::HashMap;
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

#[derive(Deserialize, Clone)]
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
    pub php_extensions: Option<Vec<String>>,
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

    let exe_metadata = fs::metadata(&exe_path).ok();
    let modified = exe_metadata.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let size = exe_metadata.as_ref().map(|m| m.len()).unwrap_or(0);
    let file_name = exe_path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let temp_dir = env::temp_dir().join(format!("exeoutput_cache_{}_{}_{}", file_name, modified, size));

    // Nettoyer les anciens caches résiduels d'anciennes exécutions/crashs
    if let Ok(entries) = fs::read_dir(env::temp_dir()) {
        let prefix = format!("exeoutput_cache_{}_", file_name);
        let current_dir_name = temp_dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with(&prefix) && name != current_dir_name {
                            log(&format!("Nettoyage de l'ancien cache résiduel : {}", name));
                            let _ = fs::remove_dir_all(path);
                        }
                    }
                }
            }
        }
    }

    let window = match WindowBuilder::new()
        .with_title(&app_name)
        .with_inner_size(wry::application::dpi::LogicalSize::new(400.0, 300.0))
        .with_visible(false)
        .with_resizable(false)
        .with_decorations(false)
        .build(&event_loop) {
            Ok(w) => w,
            Err(e) => {
                let err_msg = format!("Impossible de créer la fenêtre système : {}\nL'application va s'arrêter.", e);
                log(&err_msg);
                show_error(&err_msg);
                return;
            }
        };

    // Center window
    if let Some(monitor) = window.current_monitor() {
        let monitor_size = monitor.size();
        let window_size = window.outer_size();
        let x = (monitor_size.width as i32 - window_size.width as i32) / 2;
        let y = (monitor_size.height as i32 - window_size.height as i32) / 2;
        window.set_outer_position(wry::application::dpi::PhysicalPosition::new(x, y));
    }
    window.set_visible(true);

    let webview = match WebViewBuilder::new(window) {
        Ok(builder) => match builder.with_html(&splash_html) {
            Ok(b) => match b.build() {
                Ok(wv) => wv,
                Err(e) => {
                    let err_msg = format!("Erreur d'initialisation de WebView2 : {}\n\nVeuillez vous assurer que 'Microsoft Edge WebView2 Runtime' est bien installé sur cet ordinateur.", e);
                    log(&err_msg);
                    show_error(&err_msg);
                    return;
                }
            },
            Err(e) => {
                let err_msg = format!("Erreur configuration HTML Splash : {}", e);
                log(&err_msg);
                show_error(&err_msg);
                return;
            }
        },
        Err(e) => {
            let err_msg = format!("Erreur création WebView : {}\n\nVeuillez vérifier l'installation de WebView2.", e);
            log(&err_msg);
            show_error(&err_msg);
            return;
        }
    };

    let temp_dir_clone = temp_dir.clone();
    // Spawn extraction thread
    std::thread::spawn(move || {
        match run_background(temp_dir_clone) {
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
                    let _ = child.wait();
                }
                
                log("Nettoyage du dossier temporaire...");
                let _ = fs::remove_dir_all(&temp_dir);

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

fn run_background(temp_dir: std::path::PathBuf) -> Result<(String, Vec<std::process::Child>), Box<dyn std::error::Error + Send + Sync>> {
    let exe_path = env::current_exe()?;
    let exe_dir = exe_path.parent().ok_or("Cannot find EXE directory")?.to_path_buf();

    let mut children = Vec::new();

    // ── 1. Read and Decrypt ZIP Payload ──────────────────────────────────────────
    use std::io::{Read, Seek, SeekFrom};
    let mut file = File::open(&exe_path)?;
    
    // Seek to the last 8 bytes to get the encrypted payload length
    file.seek(SeekFrom::End(-8))?;
    let mut len_bytes = [0u8; 8];
    file.read_exact(&mut len_bytes)?;
    let payload_len = u64::from_le_bytes(len_bytes);
    
    log(&format!("Chiffrement : Taille du payload détectée = {} octets", payload_len));
    
    // Seek to the start of the encrypted payload
    let seek_offset = -(8 + payload_len as i64);
    file.seek(SeekFrom::End(seek_offset))?;
    
    let mut encrypted_payload = vec![0u8; payload_len as usize];
    file.read_exact(&mut encrypted_payload)?;
    
    // Decrypt the payload
    const ENCRYPTION_KEY: &[u8; 32] = b"ex30utput_pr0tect_key_2026_aes22";
    let decrypted_bytes = decrypt_payload(&encrypted_payload, ENCRYPTION_KEY)?;
    log("Chiffrement : Déchiffrement AES-256-GCM réussi en mémoire.");

    // ── 1b. Extraction ──────────────────────────────────────────────────────────
    fs::create_dir_all(&temp_dir)?;

    // Sécuriser l'accès aux fichiers en limitant les permissions à l'utilisateur actuel et SYSTEM
    let username = env::var("USERNAME").unwrap_or_default();
    if !username.is_empty() {
        // Exécuter icacls directement pour laisser Rust gérer le quoting automatique
        let _ = Command::new("icacls")
            .args(&[
                temp_dir.to_str().unwrap(),
                "/inheritance:r",
                "/grant",
                &format!("{}:(OI)(CI)F", username),
                "/grant",
                "*S-1-5-18:(OI)(CI)F", // SYSTEM
            ])
            .creation_flags(0x08000000)
            .status();
    }

    // Masquer le répertoire dans l'explorateur (Attribut caché et système)
    let _ = Command::new("attrib")
        .args(&["+h", "+s", temp_dir.to_str().unwrap()])
        .creation_flags(0x08000000)
        .status();

    let extraction_marker = temp_dir.join(".extraction_ok");
    if !extraction_marker.exists() {
        log("Démarrage de l'extraction...");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(decrypted_bytes))?;
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

    let php_exe = find_php(&exe_dir, &temp_dir);

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
            let _ = fs::create_dir_all(dst_in_temp.parent().unwrap_or(&dst_in_temp));
            
            // Native Rust removal is safer than cmd /c rmdir
            if dst_in_temp.exists() {
                if dst_in_temp.is_dir() {
                    let _ = fs::remove_dir_all(&dst_in_temp);
                } else {
                    let _ = fs::remove_file(&dst_in_temp);
                }
            }

            // Restauration de la logique simple (Turn 20) : Mklink via CMD standard
            // Cette version gérait correctement le dossier vendor sans conflits de guillemets.
            let dst_str = dst_in_temp.to_string_lossy();
            let src_str = src_in_data.to_string_lossy();
            
            // Suppression sécurisée avant recréation du lien
            if dst_in_temp.exists() {
                let _ = Command::new("cmd")
                    .args(&["/c", "rmdir", "/s", "/q", &dst_str])
                    .creation_flags(0x08000000)
                    .status();
            }

            // Création de la jonction
            let _ = Command::new("cmd")
                .args(&["/c", "mklink", "/j", &dst_str, &src_str])
                .creation_flags(0x08000000)
                .status();

            // FALLBACK : Si la jonction a échoué (souvent à cause de l'accent sur "Père"), 
            // on crée un dossier physique pour que Laravel ne plante pas.
            if !dst_in_temp.exists() {
                let _ = fs::create_dir_all(&dst_in_temp);
            }
        }
    }

    // ── 2b. Multiposte support: External .env override ────────────────────────
    let external_env = exe_dir.join(".env");
    if external_env.exists() {
        log("Dispositif Multiposte : Fichier .env externe détecté. Fusion robuste avec la configuration interne...");
        let target_env = temp_dir.join(".env");
        
        let mut env_map = HashMap::new();
        let mut original_order = Vec::new();

        // 1. Lire le .env interne (celui extrait du ZIP)
        if let Ok(content) = fs::read_to_string(&target_env) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    if let Some(pos) = trimmed.find('=') {
                        let key = trimmed[..pos].trim().to_string();
                        let value = trimmed[pos+1..].trim().to_string();
                        env_map.insert(key.clone(), value);
                        original_order.push(key);
                    }
                }
            }
        }

        // 2. Lire et fusionner le .env externe
        if let Ok(content) = fs::read_to_string(&external_env) {
            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    if let Some(pos) = trimmed.find('=') {
                        let key = trimmed[..pos].trim().to_string();
                        let value = trimmed[pos+1..].trim().to_string();
                        if !env_map.contains_key(&key) {
                            original_order.push(key.clone());
                        }
                        env_map.insert(key, value);
                    }
                }
            }
        }

        // 3. Réécrire le .env final
        let mut final_content = String::new();
        final_content.push_str("# Fichier généré automatiquement par le fusionneur Multiposte\n\n");
        for key in original_order {
            if let Some(val) = env_map.get(&key) {
                final_content.push_str(&format!("{}={}\n", key, val));
            }
        }
        
        let _ = fs::write(&target_env, final_content);
    }

    // ── 2c. Force APP_URL / ASSET_URL for local execution ──────────────────────
    let env_path = temp_dir.join(".env");
    if env_path.exists() {
        if let Ok(content) = fs::read_to_string(&env_path) {
            let mut env_map = std::collections::HashMap::new();
            let mut original_order = Vec::new();

            for line in content.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    if trimmed.starts_with('#') {
                        original_order.push(line.to_string());
                    } else if let Some(pos) = trimmed.find('=') {
                        let key = trimmed[..pos].trim().to_string();
                        let value = trimmed[pos+1..].trim().to_string();
                        env_map.insert(key.clone(), value);
                        original_order.push(format!("KEY:{}", key));
                    } else {
                        original_order.push(line.to_string());
                    }
                } else {
                    original_order.push("".to_string());
                }
            }

            // Force local URL configuration
            let target_url = "http://127.0.0.1:8080".to_string();
            env_map.insert("APP_URL".to_string(), target_url.clone());
            env_map.insert("ASSET_URL".to_string(), target_url.clone());
            env_map.insert("VITE_APP_URL".to_string(), target_url.clone());
            env_map.insert("MIX_APP_URL".to_string(), target_url.clone());

            for key in &["APP_URL", "ASSET_URL", "VITE_APP_URL", "MIX_APP_URL"] {
                let key_tag = format!("KEY:{}", key);
                if !original_order.contains(&key_tag) {
                    original_order.push(key_tag);
                }
            }

            let mut final_content = String::new();
            for item in original_order {
                if item.starts_with("KEY:") {
                    let key = &item[4..];
                    if let Some(val) = env_map.get(key) {
                        final_content.push_str(&format!("{}={}\n", key, val));
                    }
                } else {
                    final_content.push_str(&format!("{}\n", item));
                }
            }

            let _ = fs::write(&env_path, final_content);
            log("APP_URL et ASSET_URL forcés à http://127.0.0.1:8080 dans le fichier .env extrait.");
        }
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
                      .arg("--bind-address=0.0.0.0")
                      .arg("--max-allowed-packet=128M")
                      .arg("--innodb-buffer-pool-size=256M")
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

    log("Démarrage du serveur PHP sur 0.0.0.0:8080...");
    let mut php_cmd = new_php_command(&php_exe, &config, &temp_dir, &exe_dir);
    php_cmd.arg("-S").arg("0.0.0.0:8080").arg("-t").arg(&selected_doc_root);
    
    let server_php = temp_dir.join("server.php");
    if server_php.exists() { php_cmd.arg(&server_php); }
    
    php_cmd.creation_flags(0x08000000); 
    let child = php_cmd.spawn()?;
    children.push(child);

    // Wait for PHP and Execute SQL logic
    for _ in 0..50 {
        if std::net::TcpStream::connect("127.0.0.1:8080").is_ok() { 
            log("Le serveur PHP est prêt.");
            
            // ── 7. SQL Initialization Priority Logic ──────────────────────────────
            if db_ready {
                let db_port = config.db_port.unwrap_or(3307);
                let db_name = config.db_name.as_deref().unwrap_or("");
                let mysql_exe = data_dir.join("mysql").join("bin").join("mysql.exe");
                let init_marker = data_dir.join(".db_initialized");

                if mysql_exe.exists() && !init_marker.exists() {
                    log("Nouvelle installation détectée : démarrage de la séquence d'initialisation...");
                    
                    // A. Toujours s'assurer que la base existe (prérequis pour artisan ou import)
                    let create_sql = format!("CREATE DATABASE IF NOT EXISTS `{}` CHARACTER SET utf8mb4;", db_name);
                    let mut create_cmd = Command::new(&mysql_exe);
                    create_cmd.args(&["-u", "root", &format!("-P{}", db_port)])
                              .stdin(std::process::Stdio::piped())
                              .creation_flags(0x08000000);
                    
                    if let Ok(mut child) = create_cmd.spawn() {
                        if let Some(mut stdin) = child.stdin.take() {
                            let _ = stdin.write_all(create_sql.as_bytes());
                        }
                        let _ = child.wait();
                    }

                    // B. Vérifier si la base est déjà peuplée (tables existantes)
                    //    Si oui, on considère que c'est une réinstallation sur une base existante :
                    //    on crée le marqueur et on saute la séquence pour éviter les conflits.
                    let check_tables_sql = format!("SELECT COUNT(*) FROM information_schema.tables WHERE table_schema = '{}';", db_name);
                    let tables_check = Command::new(&mysql_exe)
                        .args(&["-u", "root", &format!("-P{}", db_port),
                               "--skip-column-names", "-e", &check_tables_sql])
                        .stdout(std::process::Stdio::piped())
                        .creation_flags(0x08000000)
                        .output();

                    let db_has_tables = if let Ok(out) = tables_check {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        stdout.trim().parse::<u32>().unwrap_or(0) > 0
                    } else {
                        false
                    };

                    // --- Phase 0 : Migration (Initialisation ou Réinitialisation) ---
                    if db_has_tables {
                        log("Base de données existante détectée. Réinitialisation complète pour assurer la compatibilité du schéma...");
                    } else {
                        log("Initialisation : Exécution des migrations de la base de données...");
                    }

                    let migrate_output = new_php_command(&php_exe, &config, &temp_dir, &exe_dir)
                        .arg("artisan").arg("migrate:fresh").arg("--force")
                        .stdout(std::process::Stdio::piped())
                        .stderr(std::process::Stdio::piped())
                        .creation_flags(0x08000000)
                        .output();
                    
                    if let Ok(output) = migrate_output {
                        let out = String::from_utf8_lossy(&output.stdout);
                        let err = String::from_utf8_lossy(&output.stderr);
                        
                        if output.status.success() {
                            log("Migrations terminées avec succès.");
                            if !out.trim().is_empty() { log(&format!("Détails migrations : {}", out)); }
                        } else {
                            log("Avertissement : La migration a rencontré des problèmes.");
                            log(&format!("Sortie standard : {}", out));
                            log(&format!("Erreur standard : {}", err));
                        }
                    }

                    let mut success = false;

                    // --- Phase 1 : Récupération Cloud (sync-pull) ---
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
                        log("Synchronisation : Tentative de récupération depuis la plateforme centrale...");
                        let pull_output = new_php_command(&php_exe, &config, &temp_dir, &exe_dir)
                            .arg("artisan").arg("parois:sync-pull")
                            .stderr(std::process::Stdio::piped())
                            .stdout(std::process::Stdio::piped())
                            .creation_flags(0x08000000)
                            .output();

                        match pull_output {
                            Ok(output) if output.status.success() => {
                                log("Données Cloud récupérées avec succès.");
                                success = true;
                            }
                            Ok(output) => {
                                let err_msg = String::from_utf8_lossy(&output.stderr);
                                let out_msg = String::from_utf8_lossy(&output.stdout);
                                log(&format!("Échec Phase 1 (Cloud) : {}{}", out_msg, err_msg));
                                log("Passage à la phase suivante...");
                            }
                            Err(e) => log(&format!("Erreur système Phase 1 : {}", e)),
                        }
                    }

                    // --- Phase 2 : Import Manuel (import.sql) ---
                    if !success {
                        let possible_names = ["import.sql", "database.sql", "db.sql", &format!("{}.sql", db_name)];
                        let mut import_target = None;
                        for name in &possible_names {
                            let candidate = exe_dir.join(*name);
                            if candidate.exists() { import_target = Some(candidate); break; }
                        }

                        if let Some(import_sql) = import_target {
                            log("Mise à jour : Fichier import.sql détecté.");
                            
                            // Utilise --force pour ignorer les avertissements SSL non bloquants
                            // et --default-character-set=utf8mb4 pour l'encodage
                            let mut import_cmd = Command::new(&mysql_exe);
                            import_cmd.args(&[
                                "-u", "root",
                                &format!("-P{}", db_port),
                                "--default-character-set=utf8mb4",
                                "--force",     // continuer même en cas d'erreurs non critiques
                                &db_name
                            ])
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .creation_flags(0x08000000);
                            
                            if let Ok(file) = File::open(&import_sql) {
                                import_cmd.stdin(std::process::Stdio::from(file));
                                match import_cmd.output() {
                                    Ok(output) => {
                                        let err_str = String::from_utf8_lossy(&output.stderr).to_string();
                                        // Filtrer les simples avertissements SSL pour ne pas les confondre avec des erreurs
                                        let has_real_errors = err_str.lines().any(|l| {
                                            let l = l.trim();
                                            l.starts_with("ERROR") || l.contains("ERROR ")
                                        });

                                        if has_real_errors {
                                            log("ERREUR : L'import SQL a echoue — des incompatibilites de schema ont ete detectees.");
                                            log(&format!("Details : {}", err_str));
                                        } else {
                                            log("Import manuel réussi (eventuels avertissements ignorés).");
                                            let _ = fs::rename(&import_sql, exe_dir.join(format!("{}.done", import_sql.file_name().unwrap().to_string_lossy())));
                                            success = true;
                                        }
                                    }
                                    Err(e) => log(&format!("Erreur système lors de l'import : {}", e)),
                                }
                            }
                        }
                    }

                    // --- Phase 3 : Initialisation interne (init.sql) ---
                    if !success {
                        let init_sql = data_dir.join("init.sql");
                        if init_sql.exists() {
                            log("Initialisation : Utilisation du fichier d'initialisation usine...");
                            let mut init_cmd = Command::new(&mysql_exe);
                            init_cmd.args(&["-u", "root", &format!("-P{}", db_port), &db_name])
                                    .creation_flags(0x08000000);
                            
                            if let Ok(file) = File::open(&init_sql) {
                                init_cmd.stdin(std::process::Stdio::from(file));
                                if init_cmd.status().map(|s| s.success()).unwrap_or(false) {
                                    log("Initialisation usine terminée.");
                                    success = true;
                                }
                            }
                        }
                    }

                    if success {
                        let _ = fs::write(&init_marker, "ok");
                        // Nettoyage final du cache pour s'assurer que les données fraîches sont vues
                        let storage_paths = [
                            temp_dir.join("storage/framework/sessions"),
                            temp_dir.join("storage/framework/views"),
                            temp_dir.join("storage/framework/cache/data"),
                        ];
                        for path in &storage_paths {
                            if path.exists() {
                                let _ = fs::remove_dir_all(path);
                                let _ = fs::create_dir_all(path);
                            }
                        }

                        // --- Phase 4 : Création du compte Administrateur par défaut ---
                        log("Vérification du compte administrateur...");
                        let admin_script = "
                            $entiteId = env('ENTITE_ID');
                            if ($entiteId && !\\App\\Models\\User::where('type', 'admin')->exists()) {
                                \\App\\Models\\User::create([
                                    'name' => 'Administrateur',
                                    'phone_number' => '0102030405',
                                    'email' => 'admin@bengespa.com',
                                    'password' => \\Illuminate\\Support\\Facades\\Hash::make('adminAdmin'),
                                    'type' => 'admin',
                                    'entite_id' => $entiteId
                                ]);
                                echo 'ADMIN_CREATED';
                            }
                        ";
                        
                        let admin_output = new_php_command(&php_exe, &config, &temp_dir, &exe_dir)
                            .arg("artisan").arg("tinker").arg("--execute").arg(admin_script.replace("\n", ""))
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .creation_flags(0x08000000)
                            .output();

                        if let Ok(output) = admin_output {
                            let out = String::from_utf8_lossy(&output.stdout);
                            if out.contains("ADMIN_CREATED") {
                                log("Compte administrateur cree : 0102030405 / adminAdmin");
                            } else {
                                log("Compte administrateur deja existant ou entite non configuree.");
                            }
                        }
                    }
                    
                    // Toujours supprimer le init.sql temporaire s'il existe pour rester propre
                    let _ = fs::remove_file(data_dir.join("init.sql"));
                }
            }

            // ── 8. Auto-Sync Background Task ──────────────────────────────────────────
            let sync_temp_dir = temp_dir.clone();
            let sync_exe_dir = exe_dir.clone();
            let sync_php_exe = php_exe.clone();
            let sync_config = config.clone();
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
                            let _ = new_php_command(&sync_php_exe, &sync_config, &sync_temp_dir, &sync_exe_dir)
                                .arg("artisan").arg("parois:sync-push")
                                .creation_flags(0x08000000)
                                .status();
                        } else {
                            log("Synchronisation automatique ignorée : ENTITE_ID non configuré ou invalide dans le .env");
                        }
                    }
                    std::thread::sleep(Duration::from_secs(900)); // 15 minutes
                }
            });

            let mut start_url = "http://127.0.0.1:8080".to_string();
            let entry = config.entry_point.trim();
            if !entry.is_empty() && entry != "index.php" && entry != "index.html" {
                if entry.starts_with('/') || entry.starts_with('?') {
                    start_url.push_str(entry);
                } else {
                    start_url.push_str("/");
                    start_url.push_str(entry);
                }
            }
            return Ok((start_url, children)); 
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    
    let mut start_url = "http://127.0.0.1:8080".to_string();
    let entry = config.entry_point.trim();
    if !entry.is_empty() && entry != "index.php" && entry != "index.html" {
        if entry.starts_with('/') || entry.starts_with('?') {
            start_url.push_str(entry);
        } else {
            start_url.push_str("/");
            start_url.push_str(entry);
        }
    }
    Ok((start_url, children))
}

fn find_php(exe_dir: &std::path::Path, temp_dir: &std::path::Path) -> std::path::PathBuf {
    // 1. Check in exe_dir/php/php.exe
    let path = exe_dir.join("php").join("php.exe");
    if path.exists() {
        log(&format!("PHP portable trouve dans le dossier de l'EXE : {}", path.display()));
        return path;
    }

    // 2. Check in exe_dir/data/php/php.exe
    let path = exe_dir.join("data").join("php").join("php.exe");
    if path.exists() {
        log(&format!("PHP portable trouve dans le dossier data de l'EXE : {}", path.display()));
        return path;
    }

    // 3. Check in temp_dir/php/php.exe (if packaged inside the ZIP)
    let path = temp_dir.join("php").join("php.exe");
    if path.exists() {
        log(&format!("PHP portable trouve dans le dossier temporaire extrait : {}", path.display()));
        return path;
    }

    // 4. Check in exe_dir/php.exe
    let path = exe_dir.join("php.exe");
    if path.exists() {
        log(&format!("PHP portable trouve directement a cote de l'EXE : {}", path.display()));
        return path;
    }

    // Fallback to system PHP
    log("Aucun PHP portable trouve. Utilisation de la commande systeme 'php'...");
    std::path::PathBuf::from("php")
}

fn decrypt_payload(encrypted_content: &[u8], key: &[u8; 32]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use aes_gcm::{Aes256Gcm, aead::{Aead, KeyInit}, Nonce};
    if encrypted_content.len() < 12 {
        return Err("Invalid ciphertext length".into());
    }
    let (nonce_bytes, ciphertext) = encrypted_content.split_at(12);
    let cipher = Aes256Gcm::new(key.into());
    let nonce = Nonce::from_slice(nonce_bytes);

    let decrypted = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption error: {}", e))?;
    Ok(decrypted)
}

fn new_php_command(
    php_exe: &std::path::Path,
    config: &Config,
    temp_dir: &std::path::Path,
    exe_dir: &std::path::Path,
) -> Command {
    let mut cmd = Command::new(php_exe);
    cmd.current_dir(temp_dir);

    // Prepend the directory of the php executable to PATH so child processes can resolve 'php'
    if let Some(php_dir) = php_exe.parent() {
        let path_env = std::env::var_os("PATH").unwrap_or_default();
        let mut paths = std::env::split_paths(&path_env).collect::<Vec<_>>();
        // Insert at the beginning to prioritize our portable PHP version
        paths.insert(0, php_dir.to_path_buf());
        if let Ok(new_path) = std::env::join_paths(paths) {
            cmd.env("PATH", new_path);
        }
    }

    // Determine the PHP ext directory (only for portable PHP)
    let is_portable = php_exe.to_string_lossy() != "php";
    let ext_dir_opt: Option<std::path::PathBuf> = if is_portable {
        php_exe.parent().map(|php_dir| php_dir.join("ext"))
    } else {
        None
    };

    // For portable PHP: use -n to ignore the bundled php.ini entirely.
    // This prevents "Unable to load dynamic library" startup warnings
    // caused by extension=xml / extension=bcmath entries in php.ini
    // that reference DLLs absent from the portable PHP distribution.
    // We will manually inject only the extensions that actually exist.
    if is_portable {
        cmd.arg("-n");
    }

    if let Some(ref ext_dir) = ext_dir_opt {
        if ext_dir.exists() {
            cmd.arg("-d").arg(format!("extension_dir={}", ext_dir.to_string_lossy()));
        }
    }

    // Enable selected PHP extensions — only if the DLL actually exists
    if let Some(exts) = &config.php_extensions {
        for ext in exts {
            let should_load = if let Some(ref ext_dir) = ext_dir_opt {
                if ext_dir.exists() {
                    // Try php_<ext>.dll first (Windows standard), then bare <ext>
                    let dll_with_prefix = ext_dir.join(format!("php_{}.dll", ext));
                    let dll_bare       = ext_dir.join(format!("{}.dll", ext));
                    let dll_no_ext     = ext_dir.join(ext.as_str());
                    dll_with_prefix.exists() || dll_bare.exists() || dll_no_ext.exists()
                } else {
                    true // no ext_dir → let PHP decide (system install)
                }
            } else {
                true // system PHP → pass all extensions as-is
            };

            if should_load {
                cmd.arg("-d").arg(format!("extension={}", ext));
            }
        }
    }

    // ── Configure SSL certificates for curl/openssl ──────────────────────────
    // Build a prioritised list of candidate cacert.pem locations:
    //   1. Next to the final EXE (user can place one there)
    //   2. Next to php.exe (standard portable PHP location)
    //   3. Common system-level locations used by curl on Windows
    let mut cacert_candidates: Vec<std::path::PathBuf> = Vec::new();

    // (a) next to the deployed EXE
    cacert_candidates.push(exe_dir.join("cacert.pem"));
    cacert_candidates.push(exe_dir.join("php").join("cacert.pem"));

    // (b) next to php.exe
    if let Some(php_dir) = php_exe.parent() {
        cacert_candidates.push(php_dir.join("cacert.pem"));
        cacert_candidates.push(php_dir.join("ssl").join("cacert.pem"));
        cacert_candidates.push(php_dir.join("extras").join("ssl").join("cacert.pem"));
        // Some PHP Windows builds ship it here
        cacert_candidates.push(php_dir.join("ca-bundle.crt"));
        cacert_candidates.push(php_dir.join("curl-ca-bundle.crt"));
    }

    // (c) well-known system paths on Windows
    if let Ok(system_root) = std::env::var("SystemRoot") {
        cacert_candidates.push(std::path::PathBuf::from(&system_root).join("System32").join("curl-ca-bundle.crt"));
    }
    if let Ok(program_files) = std::env::var("ProgramFiles") {
        cacert_candidates.push(std::path::PathBuf::from(&program_files).join("curl").join("cacert.pem"));
    }

    let found_cacert = cacert_candidates.iter().find(|p| p.exists()).cloned();

    if let Some(ref cacert) = found_cacert {
        let path_str = cacert.to_string_lossy();
        // PHP ini settings (for openssl/curl PHP extensions)
        cmd.arg("-d").arg(format!("curl.cainfo={}", path_str));
        cmd.arg("-d").arg(format!("openssl.cafile={}", path_str));
        // Environment variables picked up natively by curl and OpenSSL
        cmd.env("CURL_CA_BUNDLE", cacert.as_os_str());
        cmd.env("SSL_CERT_FILE", cacert.as_os_str());
        cmd.env("REQUESTS_CA_BUNDLE", cacert.as_os_str()); // Python Guzzle compat
    } else {
        // Aucun cacert.pem trouvé — désactiver la vérification SSL comme dernier recours
        // pour ne pas bloquer la synchronisation cloud.
        // Note : placer un cacert.pem à côté de l'EXE ou dans php/ restaure la validation complète.
        cmd.arg("-d").arg("curl.cainfo=");
        // Désactiver la vérification SSL pour Guzzle (utilisé par Laravel pour les requêtes HTTP)
        cmd.env("GUZZLE_VERIFY", "false");
        // Variable interprétée par certaines configurations de Guzzle personnalisées
        cmd.env("APP_VERIFY_SSL", "false");
        // Désactive la vérification côté openssl pour les connexions sortantes
        cmd.env("SSL_NO_VERIFY", "1");
    }

    // Force local APP_URL / ASSET_URL
    cmd.env("APP_URL", "http://127.0.0.1:8080");
    cmd.env("ASSET_URL", "http://127.0.0.1:8080");

    // Configure Database connection parameters
    if let Some(db_type) = &config.db_type {
        if db_type != "none" {
            cmd.env("DB_CONNECTION", if db_type == "mariadb" { "mysql" } else { db_type });
            cmd.env("DB_HOST", "127.0.0.1");
            cmd.env("DB_PORT", config.db_port.unwrap_or(3307).to_string());
            
            let db_name = config.db_name.as_deref().unwrap_or("");
            if db_type == "sqlite" {
                let file_name = std::path::Path::new(db_name).file_name().unwrap_or_default();
                let external_sqlite = exe_dir.join(file_name);
                if external_sqlite.exists() {
                     cmd.env("DB_DATABASE", external_sqlite.to_str().unwrap());
                } else if exe_dir.join("database.sqlite").exists() {
                     cmd.env("DB_DATABASE", exe_dir.join("database.sqlite").to_str().unwrap());
                } else {
                     cmd.env("DB_DATABASE", db_name);
                }
            } else {
                cmd.env("DB_DATABASE", db_name);
            }
            
            cmd.env("DB_USERNAME", config.db_user.as_deref().unwrap_or("root"));
            cmd.env("DB_PASSWORD", config.db_pass.as_deref().unwrap_or(""));
        }
    }

    cmd
}
