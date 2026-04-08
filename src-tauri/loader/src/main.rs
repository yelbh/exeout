#![windows_subsystem = "windows"]

use std::env;
use std::fs::{self, File};
use std::io::{self, Write, BufRead, BufReader, Read};
use std::net::TcpStream;
use std::process::{Command, Stdio};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::time::UNIX_EPOCH;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use serde::Deserialize;
use tao::dpi::LogicalSize;

use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::webview::WebViewBuilder;

#[derive(Deserialize)]
struct Config {
    entry_point: String,
    pub public_dir: Option<String>,
    pub external_dirs: Option<Vec<String>>,
    pub db_type: Option<String>,
    pub db_port: Option<u32>,
    pub db_name: Option<String>,
    pub db_user: Option<String>,
    pub db_pass: Option<String>,
    pub has_init_sql: Option<bool>,
    pub version: Option<String>,
    pub update_url: Option<String>,
}

#[derive(Deserialize)]
struct UpdateManifest {
    version: String,
    notes: Option<String>,
    platforms: std::collections::HashMap<String, PlatformInfo>,
}

#[derive(Deserialize)]
struct PlatformInfo {
    url: String,
}

#[derive(Debug)]
enum UserEvent {
    Ready(String),
    FatalError(String)
}

fn log(msg: &str) {
    if let Ok(mut path) = env::current_exe() {
        path.set_extension("log");
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), msg);
        }
    }
}

fn save_external_env(updates: std::collections::HashMap<String, String>) -> io::Result<()> {
    let exe_path = env::current_exe()?;
    let env_path = exe_path.parent().unwrap().join(".env");
    
    let mut lines = Vec::new();
    let mut updated_keys = std::collections::HashSet::new();

    if env_path.exists() {
        let file = File::open(&env_path)?;
        let reader = BufReader::new(file);
        for line_res in reader.lines() {
            let line = line_res?;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                lines.push(line);
                continue;
            }

            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim().to_string();
                if let Some(new_val) = updates.get(&key) {
                    lines.push(format!("{}={}", key, new_val));
                    updated_keys.insert(key);
                } else {
                    lines.push(line);
                }
            } else {
                lines.push(line);
            }
        }
    }

    for (key, val) in updates {
        if !updated_keys.contains(&key) {
            lines.push(format!("{}={}", key, val));
        }
    }

    fs::write(env_path, lines.join("\n"))?;
    Ok(())
}

fn load_external_env() -> HashMap<String, String> {
    let mut config = HashMap::new();
    if let Ok(exe_path) = env::current_exe() {
        let env_path = exe_path.parent().unwrap().join(".env");
        if env_path.exists() {
            if let Ok(file) = File::open(env_path) {
                let reader = BufReader::new(file);
                for line_res in reader.lines() {
                    if let Ok(line) = line_res {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() && !trimmed.starts_with('#') {
                            if let Some(pos) = trimmed.find('=') {
                                let key = trimmed[..pos].trim().to_string();
                                let val = trimmed[pos + 1..].trim().to_string();
                                config.insert(key, val);
                            }
                        }
                    }
                }
            }
        }
    }
    config
}

fn update_env_file(path: &std::path::Path, updates: Vec<(&str, String)>) -> io::Result<()> {
    let mut new_lines = Vec::new();
    let mut keys_to_add: std::collections::HashSet<String> = updates.iter().map(|(k, _)| k.to_string()).collect();

    if path.exists() {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        for line_res in reader.lines() {
            let line = line_res?;
            let trimmed = line.trim();
            let mut matched = false;
            for (key, value) in &updates {
                let prefix = format!("{}=", key);
                if trimmed.starts_with(&prefix) {
                    new_lines.push(format!("{}={}", key, value));
                    keys_to_add.remove(*key);
                    matched = true;
                    break;
                }
            }
            if !matched {
                new_lines.push(line);
            }
        }
    }

    for key in keys_to_add {
        for (k, v) in &updates {
            if *k == key {
                new_lines.push(format!("{}={}", k, v));
            }
        }
    }

    let mut file = File::create(path)?;
    for line in new_lines {
        writeln!(file, "{}", line)?;
    }
    Ok(())
}

fn main() {
    log("=== Lancement de l'application ===");
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
    let exe_file = File::open(&exe_path)
        .map_err(|e| format!("Impossible d'ouvrir l'exécutable : {}", e))?;
    
    let exe_metadata = fs::metadata(&exe_path)?;
    let modified = exe_metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs();
    let size = exe_metadata.len();
    let file_name = exe_path.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let window_title = file_name.clone();

    let php_process: Arc<Mutex<Option<std::process::Child>>> = Arc::new(Mutex::new(None));
    let db_process: Arc<Mutex<Option<std::process::Child>>> = Arc::new(Mutex::new(None));

    let php_clone = Arc::clone(&php_process);
    let db_clone = Arc::clone(&db_process);

    let event_loop = EventLoop::<UserEvent>::with_user_event();
    let proxy = event_loop.create_proxy();
    let proxy_ipc = proxy.clone();

    let window = WindowBuilder::new()
        .with_title(&window_title)
        .with_visible(true)
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
        .with_maximized(true)
        .build(&event_loop)?;
    log("Fenêtre native créée");

    let splash_html = r#"
        <!DOCTYPE html>
        <html>
        <head><meta charset="UTF-8"></head>
        <body style='display:flex;flex-direction:column;justify-content:center;align-items:center;height:100vh;margin:0;font-family:system-ui, sans-serif;background-color:#f8fafc;color:#0f172a;'>
            <div style='width:40px;height:40px;border:4px solid #e2e8f0;border-top-color:#3b82f6;border-radius:50%;animation:spin 1s linear infinite;margin-bottom:20px;'></div>
            <h2 style='margin:0;font-weight:600;'>Démarrage en cours...</h2>
            <p style='color:#64748b;margin-top:8px;'>Préparation de l'environnement de l'application</p>
            <p style='color:#cbd5e1;font-size:10px;margin-top:20px;'>Build 2026-03-30 11:30 (Native Wry)</p>
            <style>@keyframes spin { 100% { transform: rotate(360deg); } }</style>
        </body>
        </html>
    "#;

    let webview = WebViewBuilder::new(window)?
        .with_html(splash_html)?
        .with_initialization_script(r#"
            window.addEventListener('keydown', (e) => {
                if (e.ctrlKey && e.shiftKey && e.key === 'S') {
                    window.ipc.postMessage('open_settings');
                }
            });
        "#)
        .with_ipc_handler(move |_, req| {
            if req == "open_settings" {
                let _ = proxy_ipc.send_event(UserEvent::Ready("SETTINGS".to_string()));
            } else if req.starts_with("save_settings:") {
                let json = &req["save_settings:".len()..];
                if let Ok(updates) = serde_json::from_str::<std::collections::HashMap<String, String>>(json) {
                    let _ = save_external_env(updates);
                    // Reload the app to apply changes
                    let _ = proxy_ipc.send_event(UserEvent::Ready("RELOAD".to_string()));
                }
            }
        })
        .build()?;

    // Background Worker
    std::thread::spawn(move || {
        let result = (|| -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            let mut archive = zip::ZipArchive::new(exe_file)
                .map_err(|_| "Aucun contenu de projet trouvé dans cet exécutable. (ZIP manquant)")?;
            
            let temp_dir = env::temp_dir().join(format!("exeoutput_cache_{}_{}_{}", file_name, modified, size));
            log(&format!("Extraction vers {}", temp_dir.display()));
            fs::create_dir_all(&temp_dir)?;
            
            let extraction_marker = temp_dir.join(".extraction_ok");
            
            // Extract in Parallel
            if !extraction_marker.exists() {
                log("Début de l'extraction des fichiers du projet (Multi-threaded)...");
                let start_time = std::time::Instant::now();
                let total_files = archive.len();
                
                // Get number of CPUs for optimal parallelism
                let num_threads = num_cpus::get().max(1);
                log(&format!("Utilisation de {} threads pour l'extraction.", num_threads));
                
                let temp_dir_shared = Arc::new(temp_dir.clone());
                let total_done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
                
                let mut handles = vec![];
                let files_per_thread = (total_files + num_threads - 1) / num_threads;
                
                for t in 0..num_threads {
                    let start_idx = t * files_per_thread;
                    let end_idx = ((t + 1) * files_per_thread).min(total_files);
                    if start_idx >= total_files { break; }
                    
                    let exe_path_clone = exe_path.clone();
                    let temp_dir_clone = Arc::clone(&temp_dir_shared);
                    let total_done_clone = Arc::clone(&total_done);
                    
                    let handle = std::thread::spawn(move || {
                        // Each thread opens its own file handle for parallel reading
                        if let Ok(file) = File::open(&exe_path_clone) {
                            if let Ok(mut thread_archive) = zip::ZipArchive::new(file) {
                                for i in start_idx..end_idx {
                                    let done = total_done_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                                    if done % 1000 == 0 {
                                        // Logging from threads can be noisy, but let's keep one thread doing it
                                        // if t == 0 { log(&format!("Extraction en cours : {}/{}...", done, total_files)); }
                                    }
                                    
                                    let mut file = match thread_archive.by_index(i) {
                                        Ok(f) => f,
                                        Err(_) => continue,
                                    };
                                    
                                    let outpath = match file.enclosed_name() {
                                        Some(path) => temp_dir_clone.join(path),
                                        None => continue,
                                    };
                                    
                                    if file.name().ends_with('/') {
                                        let _ = fs::create_dir_all(&outpath);
                                    } else {
                                        if let Some(p) = outpath.parent() {
                                            if !p.exists() {
                                                let _ = fs::create_dir_all(p);
                                            }
                                        }
                                        if let Ok(mut outfile) = File::create(&outpath) {
                                            let _ = io::copy(&mut file, &mut outfile);
                                        }
                                    }
                                }
                            }
                        }
                    });
                    handles.push(handle);
                }
                
                // Monitor progress from main worker thread
                while handles.iter().any(|h| !h.is_finished()) {
                    let done = total_done.load(std::sync::atomic::Ordering::Relaxed);
                    log(&format!("Extraction en cours : {}/{} fichiers...", done, total_files));
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                }
                
                for h in handles { let _ = h.join(); }
                
                // Assurer que les dossiers storage essentiels existent pour Laravel
                let storage_framework = temp_dir.join("storage").join("framework");
                let _ = fs::create_dir_all(storage_framework.join("sessions"));
                let _ = fs::create_dir_all(storage_framework.join("views"));
                let _ = fs::create_dir_all(storage_framework.join("cache"));
                let _ = fs::create_dir_all(temp_dir.join("storage").join("logs"));
                
                log(&format!("Extraction terminée en {} ms ({} fichiers)", start_time.elapsed().as_millis(), total_files));
                let _ = fs::write(&extraction_marker, "ok");
            } else {
                log("Utilisation du cache d'extraction existant.");
            }
            
            let config_path = temp_dir.join("exeoutput.json");
            let (entry_point, public_dir, external_dirs, db_type, db_port, db_name, db_user, db_pass, has_init_sql, version, update_url) = if config_path.exists() {
                let content = fs::read_to_string(&config_path)?;
                let config: Config = serde_json::from_str(&content)
                    .map_err(|e| format!("Erreur de configuration (JSON invalide) : {}", e))?;
                (
                    config.entry_point,
                    config.public_dir.unwrap_or_default(),
                    config.external_dirs.unwrap_or_default(),
                    config.db_type.unwrap_or_else(|| "none".to_string()),
                    config.db_port.unwrap_or(3307),
                    config.db_name.unwrap_or_default(),
                    config.db_user.unwrap_or_else(|| "root".to_string()),
                    config.db_pass.unwrap_or_default(),
                    config.has_init_sql.unwrap_or(false),
                    config.version,
                    config.update_url
                )
            } else {
                ("index.php".to_string(), "".to_string(), vec![], "none".to_string(), 3307, "".to_string(), "root".to_string(), "".to_string(), false, None, None)
            };

            if let Some(ref update_url) = update_url {
                if let Some(ref current_version) = version {
                    log(&format!("Vérification des mises à jour sur l'URL: {}", update_url));
                    if let Ok(resp) = ureq::get(update_url).timeout(std::time::Duration::from_secs(5)).call() {
                        if let Ok(manifest) = resp.into_json::<UpdateManifest>() {
                            if manifest.version != *current_version {
                                log(&format!("Nouvelle version trouvée: {} (actuelle: {})", manifest.version, current_version));
                                
                                let notes = manifest.notes.as_deref().unwrap_or("Aucune note de version fournie.");
                                let confirm = rfd::MessageDialog::new()
                                    .set_title("Mise à jour disponible")
                                    .set_description(&format!("Une nouvelle version ({}) est disponible.\n\nNotes :\n{}\n\nVoulez-vous l'installer maintenant ?", manifest.version, notes))
                                    .set_buttons(rfd::MessageButtons::YesNo)
                                    .show();

                                if confirm == rfd::MessageDialogResult::Yes {
                                    if let Some(platform) = manifest.platforms.get("windows-x86_64") {
                                        let _ = proxy.send_event(UserEvent::Ready("Téléchargement de la mise à jour...".to_string()));
                                        log(&format!("Téléchargement depuis : {}", platform.url));
                                        
                                        let temp_exe = std::env::temp_dir().join(format!("update_{}_{}.exe", file_name, manifest.version));
                                        if let Ok(mut file) = std::fs::File::create(&temp_exe) {
                                            if let Ok(dl_resp) = ureq::get(&platform.url).call() {
                                                let mut reader = dl_resp.into_reader();
                                                if std::io::copy(&mut reader, &mut file).is_ok() {
                                                    log("Mise à jour téléchargée. Remplacement en cours...");
                                                    if self_replace::self_replace(&temp_exe).is_ok() {
                                                        log("Remplacement réussi. Redémarrage de l'application.");
                                                        let _ = std::process::Command::new(&exe_path).spawn();
                                                        std::process::exit(0);
                                                    } else {
                                                        log("Échec du remplacement de l'exécutable.");
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !external_dirs.is_empty() {
                let exe_dir = exe_path.parent().unwrap();
                let data_dir = exe_dir.join("data");
                
                for dir in external_dirs {
                    let src = data_dir.join(&dir);
                    let dst = temp_dir.join(&dir);
                    
                    if src.exists() {
                        if dst.exists() {
                           let _ = fs::remove_dir_all(&dst);
                        }
                        
                        let _ = Command::new("cmd")
                            .args(&["/c", "mklink", "/j", dst.to_str().unwrap(), src.to_str().unwrap()])
                            .creation_flags(0x08000000)
                            .status();
                    }
                }
            }

            log(&format!("Type de base de données configuré : {}", db_type));
            if db_type == "mariadb" {
                let exe_dir = exe_path.parent().unwrap();
                let mysql_dir = exe_dir.join("data").join("mysql");
                let mysqld_exe = mysql_dir.join("bin").join("mysqld.exe");
                let data_dir = mysql_dir.join("data");
                
                log(&format!("Vérification MariaDB dans : {}", mysql_dir.display()));
                
                if mysqld_exe.exists() {
                    if !data_dir.exists() {
                        log("Le dossier 'data' de MariaDB n'existe pas. Création en cours...");
                        let _ = fs::create_dir_all(&data_dir);
                        
                        let install_db_exe = mysql_dir.join("bin").join("mysql_install_db.exe");
                        if install_db_exe.exists() {
                            log("Initialisation des bases de données système (mysql_install_db)...");
                            let mut install_cmd = Command::new(&install_db_exe);
                            install_cmd.arg(format!("--datadir={}", data_dir.to_str().unwrap()));
                            #[cfg(windows)] install_cmd.creation_flags(0x08000000);
                            
                            if let Ok(mut child) = install_cmd.spawn() {
                                let _ = child.wait();
                                log("Initialisation du dossier data terminée.");
                            } else {
                                log("ERREUR : Impossible de lancer mysql_install_db.exe");
                            }
                        } else {
                            log("AVERTISSEMENT : mysql_install_db.exe est manquant, MariaDB risque de planter.");
                        }
                    }
                    // Vérifier si le port est déjà occupé
                    if TcpStream::connect(("127.0.0.1", db_port as u16)).is_ok() {
                        log(&format!("AVERTISSEMENT : Le port {} est déjà utilisé par un autre processus. MariaDB pourrait ne pas démarrer.", db_port));
                    }

                    let db_log_path = temp_dir.join("mariadb_output.log");

                    let mut db_cmd = Command::new(&mysqld_exe);
                    db_cmd.arg("--no-defaults")
                          .arg(format!("--datadir={}", data_dir.to_str().unwrap()))
                          .arg(format!("--tmpdir={}", temp_dir.to_str().unwrap()))
                          .arg(format!("--port={}", db_port))
                          .arg("--bind-address=127.0.0.1")
                          .arg("--skip-grant-tables")
                          .arg("--console")
                          .arg(format!("--log-error={}", db_log_path.to_str().unwrap()));

                    #[cfg(windows)]
                    db_cmd.creation_flags(0x08000000);

                    log(&format!("Démarrage de MariaDB Portable sur le port {}...", db_port));
                    log(&format!("Répertoire temporaire (tmpdir) : {}", temp_dir.display()));
                    log(&format!("Log d'erreurs MariaDB : {}", db_log_path.display()));

                    // Rediriger également stdout/stderr vers notre log
                    let db_log_file = File::create(&db_log_path).ok();
                    
                    if let Some(f) = db_log_file {
                         db_cmd.stdout(Stdio::from(f.try_clone().unwrap()))
                               .stderr(Stdio::from(f));
                    }
                    
                    match db_cmd.spawn() {
                        Ok(child) => {
                             log("Processus MariaDB lancé avec succès.");
                             *db_clone.lock().unwrap() = Some(child);
                        },
                        Err(e) => {
                             log(&format!("ERREUR : Impossible de lancer MariaDB : {}", e));
                        }
                    }

                    // On attend la disponibilité de MariaDB AVANT les imports (30s max)
                    let mut connected = false;
                    for _ in 0..300 {
                        if TcpStream::connect(("127.0.0.1", db_port as u16)).is_ok() {
                            connected = true;
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(100));
                    }
                    
                    if connected {
                        log(&format!("Connexion à MariaDB établie sur 127.0.0.1:{}", db_port));
                    } else {
                        log("AVERTISSEMENT : Échec de la connexion TCP à MariaDB après 10 secondes.");
                    }

                    // Créer la base de données si elle n'existe pas encore
                    if connected {
                        let mysql_cli = mysql_dir.join("bin").join("mysql.exe");
                        if mysql_cli.exists() {
                            let create_db_sql = format!("CREATE DATABASE IF NOT EXISTS `{}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;", db_name);
                            let mut create_cmd = Command::new(&mysql_cli);
                            create_cmd.args(&["-u", &db_user, &format!("-P{}", db_port), "-e", &create_db_sql]);
                            #[cfg(windows)] create_cmd.creation_flags(0x08000000);
                            match create_cmd.status() {
                                Ok(s) if s.success() => log(&format!("Base de données '{}' créée ou déjà existante.", db_name)),
                                Ok(s) => log(&format!("AVERTISSEMENT : Création de la BDD - code: {}", s)),
                                Err(e) => log(&format!("ERREUR lors de la création de la BDD : {}", e)),
                            }
                        }
                    }

                    let init_marker = mysql_dir.join(".initialized_db");
                    let init_sql_path = exe_dir.join("data").join("init.sql");
                    let client_import_sql = exe_dir.join("import.sql");
                    
                    if connected && has_init_sql && !init_marker.exists() && init_sql_path.exists() {
                        log("Importation de init.sql (Première installation)...");
                        let mysql_cli = mysql_dir.join("bin").join("mysql.exe");
                        if mysql_cli.exists() {
                            {
                                if let Ok(file) = File::open(&init_sql_path) {
                                    let mut import_cmd = Command::new(&mysql_cli);
                                    import_cmd.args(&["-u", &db_user, &format!("-P{}", db_port), &db_name])
                                              .stdin(Stdio::from(file));
                                    #[cfg(windows)] import_cmd.creation_flags(0x08000000);
                                    let _ = import_cmd.status();
                                }
                            }
                            let _ = fs::write(&init_marker, "initialized");
                        }
                    }

                    if connected && client_import_sql.exists() {
                        log("Détection de import.sql, lancement de l'import...");
                        let mysql_cli = mysql_dir.join("bin").join("mysql.exe");
                        if mysql_cli.exists() {
                            {
                                if let Ok(file) = File::open(&client_import_sql) {
                                    let mut import_cmd = Command::new(&mysql_cli);
                                    import_cmd.args(&["-u", &db_user, &format!("-P{}", db_port), &db_name])
                                              .stdin(Stdio::from(file));
                                    #[cfg(windows)] import_cmd.creation_flags(0x08000000);
                                    let _ = import_cmd.status();
                                }
                            }
                            
                            let done_path = exe_dir.join("import.sql.done");
                            if done_path.exists() {
                                let _ = fs::remove_file(&done_path);
                            }
                            let _ = fs::rename(&client_import_sql, &done_path);
                            log("import.sql importé et renommé en .done");
                        }
                    }
                } else {
                    log("ERREUR : mysqld.exe introuvable dans data/mysql/bin/ !");
                }
            }

            let mut web_root = temp_dir.clone();
            let mut using_public_subfolder = false;
            
            let maybe_public = temp_dir.join("public");
            if maybe_public.exists() && maybe_public.is_dir() {
                web_root = maybe_public;
                using_public_subfolder = true;
                log("Dossier 'public' détecté et utilisé comme racine web (Standard Laravel).");
            } else if !public_dir.is_empty() {
                web_root = temp_dir.join(&public_dir);
                if public_dir == "public" { using_public_subfolder = true; }
            }
            
            let entry_path = web_root.join(&entry_point);
            // Si le point d'entrée n'est pas dans la racine web mais à la racine du projet, on ajuste
            let final_entry_path = if !entry_path.exists() && using_public_subfolder {
                temp_dir.join(&entry_point)
            } else {
                entry_path
            };
            
            if !final_entry_path.exists() {
                return Err(format!("Le point d'entrée '{}' est introuvable (tenté dans {}).", entry_point, web_root.display()).into());
            }
            
            // Fix double public/ entry point in URL
            // Mise à jour intelligente du fichier .env
            let env_file = temp_dir.join(".env");
            let external_env = exe_path.parent().unwrap().join(".env");
            let mut using_external_env = false;
            
            if external_env.exists() {
                log("Fichier .env externe détecté à côté de l'exécutable. Application prioritaire...");
                if fs::copy(&external_env, &env_file).is_ok() {
                    using_external_env = true;
                    log(".env externe appliqué avec succès.");
                }
            }

            if db_type == "mariadb" && !using_external_env {
                let mut updates = vec![
                    ("DB_CONNECTION", "mysql".to_string()),
                    ("DB_HOST", "127.0.0.1".to_string()),
                    ("DB_PORT", db_port.to_string()),
                    ("DB_DATABASE", db_name.clone()),
                    ("DB_USERNAME", db_user.clone()),
                    ("DB_PASSWORD", db_pass.clone()),
                    ("APP_URL", "http://127.0.0.1:8080".to_string()),
                    ("APP_DEBUG", "true".to_string()),
                    ("SESSION_DRIVER", "file".to_string()),
                    ("SESSION_SECURE_COOKIE", "false".to_string()),
                    ("SESSION_DOMAIN", "null".to_string()),
                    ("SESSION_SAME_SITE", "null".to_string()),
                    ("SESSION_PATH", "/".to_string()),
                    ("SESSION_LIFETIME", "120".to_string()),
                ];

                if let Err(e) = update_env_file(&env_file, updates) {
                    log(&format!("Avertissement : Impossible de mettre à jour le fichier .env : {}", e));
                } else {
                    log("Fichier .env mis à jour avec les paramètres de la base de données et les diagnostics.");
                }
            }
            
            // Vérifier si MariaDB est toujours en vie
            if db_type == "mariadb" {
                if let Ok(mut lock) = db_clone.lock() {
                    if let Some(child) = lock.as_mut() {
                        match child.try_wait() {
                            Ok(Some(status)) => {
                                log(&format!("ERREUR : MariaDB s'est arrêté prématurément (Code: {})", status));
                                // DUMP LOGS
                                let db_log_path = temp_dir.join("mariadb_output.log");
                                if db_log_path.exists() {
                                    if let Ok(mut f) = File::open(&db_log_path) {
                                        let mut content = String::new();
                                        let _ = f.read_to_string(&mut content);
                                        log("--- CONTENU DE MARIADB_OUTPUT.LOG ---");
                                        log(&content);
                                        log("--- FIN DU LOG MARIADB ---");
                                    }
                                }
                            },
                            Ok(None) => log("MariaDB est toujours en cours d'exécution."),
                            Err(e) => log(&format!("Erreur lors de la vérification de MariaDB : {}", e)),
                        }
                    }
                }
            }
            
            let session_path = temp_dir.join("storage").join("framework").join("sessions");
            let _ = fs::create_dir_all(&session_path);

            let mut cmd = Command::new("php");
            cmd.arg("-S").arg("127.0.0.1:8080")
               .arg("-t").arg(&web_root);
               
            let server_php = temp_dir.join("server.php");
            if server_php.exists() {
                cmd.arg(server_php);
            }

            cmd.arg("-d").arg(format!("session.save_path={}", session_path.to_str().unwrap()))
               .arg("-d").arg("display_errors=1")
               .arg("-d").arg("error_reporting=E_ALL")
               .current_dir(&temp_dir);
               
            if db_type == "mariadb" && !using_external_env {
                cmd.env("DB_HOST", "127.0.0.1");
                cmd.env("DB_PORT", db_port.to_string());
                cmd.env("DB_DATABASE", &db_name);
                cmd.env("DB_USERNAME", &db_user);
                cmd.env("DB_PASSWORD", &db_pass);
            } else if db_type == "sqlite" && !using_external_env {
                let db_path = temp_dir.join("database.sqlite");
                cmd.env("DB_DATABASE", db_path.to_str().unwrap());
            }

            #[cfg(windows)]
            cmd.creation_flags(0x08000000); 

            log("Démarrage du serveur PHP interne...");
            let child = cmd.spawn()
                .map_err(|e| format!("Impossible de lancer PHP.\nErreur: {}", e))?;
            *php_clone.lock().unwrap() = Some(child);
            
            // Fix double public/ entry point in URL
            // Fix double public/ entry point in URL
            let mut final_entry = entry_point.clone();
            
            // Si on utilise le dossier public comme racine, l'URL ne doit pas contenir "public/"
            if using_public_subfolder {
                if final_entry.starts_with("public/") {
                    final_entry = final_entry["public/".len()..].to_string();
                } else if final_entry.starts_with("public\\") {
                    final_entry = final_entry["public\\".len()..].to_string();
                }
            }
            
            let target_url = format!("http://127.0.0.1:8080/{}", final_entry.replace('\\', "/"));
            
            // --- DIAGNOSTIC AUTOMATIQUE ---
            let health_script_path = web_root.join("health_check_internal.php");
            let db_check = if db_type != "none" {
                format!(r#"
try {{
    $pdo = new PDO("mysql:host=$db_host;port=$db_port;dbname=$db_name", $db_user, $db_pass, [
        PDO::ATTR_ERRMODE => PDO::ERRMODE_EXCEPTION
    ]);
    echo "DB: Connexion REUSSIE\n";
    
    $stmt = $pdo->query("SHOW TABLES LIKE 'users'");
    if ($stmt->rowCount() > 0) {{
        $stmt = $pdo->query("SELECT COUNT(*) FROM users");
        $count = $stmt->fetchColumn();
        echo "DB: Table 'users' trouvee, nombre d'entrees = $count\n";
    }} else {{
        echo "DB: ATTENTION - Table 'users' INCONNUE !\n";
    }}
}} catch (Exception $e) {{
    echo "DB: ERREUR = " . $e->getMessage() . "\n";
}}
"#)
            } else {
                "echo \"DB: Desactivce (Type: none)\n\";".to_string()
            };

            let health_script_content = format!(r#"<?php
$db_host = '127.0.0.1';
$db_port = '{}';
$db_user = '{}';
$db_pass = '{}';
$db_name = '{}';

echo "\n--- RAPPORT DE DIAGNOSTIC ---\n";
{}

$storage = __DIR__ . '/../storage/framework/sessions';
if (!is_dir($storage)) {{
    echo "STORAGE: Dossier sessions absent ($storage)\n";
}} elseif (is_writable($storage)) {{
    echo "STORAGE: Dossier sessions OK (Writable)\n";
}} else {{
    echo "STORAGE: Dossier sessions NON-WRITABLE !\n";
}}

$env_path = __DIR__ . '/../.env';
$env = @file_get_contents($env_path);
if (preg_match('/APP_KEY=/', $env)) {{
    echo "ENV: APP_KEY Detectee\n";
}} else {{
    echo "ENV: ATTENTION - APP_KEY ABSENTE !\n";
}}
echo "--- FIN DU DIAGNOSTIC ---\n\n";
"#, db_port, db_user, db_pass, db_name, db_check);
            
            let _ = fs::write(&health_script_path, health_script_content);
            
            // Lancer le diagnostic immédiatement via PHP CLI
            log("Lancement du diagnostic interne...");
            let mut health_cmd = Command::new("php");
            health_cmd.arg(&health_script_path).current_dir(&temp_dir);
            #[cfg(windows)] health_cmd.creation_flags(0x08000000);
            if let Ok(output) = health_cmd.output() {
                let report = String::from_utf8_lossy(&output.stdout);
                log(&report);
            }
            let _ = fs::remove_file(&health_script_path);
            
            log(&format!("URL prête : {}", target_url));
            log(&format!("NOTE: Si besoin, Adminer est dispo sur http://127.0.0.1:8080/adminer.php (si inclus dans le projet)."));
            Ok(target_url)
        })();

        match result {
            Ok(url) => { let _ = proxy.send_event(UserEvent::Ready(url)); }
            Err(e) => { let _ = proxy.send_event(UserEvent::FatalError(e.to_string())); }
        }
    });

    fn location_reload(webview: &wry::webview::WebView) {
        let _ = webview.evaluate_script("location.reload()");
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::Ready(url)) => {
                if url == "RELOAD" {
                   location_reload(&webview);
                } else if url == "SETTINGS" {
                    let _ = webview.window().set_title("Configuration du Poste");
                    let settings_html = r#"
                        <!DOCTYPE html>
                        <html>
                        <head>
                            <meta charset="UTF-8">
                            <style>
                                body { font-family: system-ui; padding: 20px; background: #f8fafc; color: #1e293b; }
                                .card { background: white; padding: 20px; border-radius: 8px; box-shadow: 0 4px 6px -1px rgb(0 0 0 / 0.1); max-width: 600px; margin: auto; }
                                h1 { font-size: 1.25rem; margin-top: 0; display: flex; justify-content: space-between; align-items: center; }
                                .form-group { margin-bottom: 15px; display: flex; gap: 10px; align-items: flex-end; }
                                .field-col { flex: 1; }
                                label { display: block; margin-bottom: 5px; font-size: 0.875rem; color: #64748b; }
                                input { width: 100%; padding: 8px; border: 1px solid #e2e8f0; border-radius: 4px; box-sizing: border-box; font-family: monospace; }
                                button { background: #3b82f6; color: white; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer; font-weight: 600; }
                                .btn-save { width: 100%; margin-top: 20px; }
                                .btn-add { background: #10b981; padding: 5px 10px; font-size: 0.75rem; }
                                .btn-remove { background: #ef4444; padding: 8px; margin-bottom: 0; }
                                .back-btn { background: transparent; color: #64748b; border: 1px solid #e2e8f0; margin-top: 10px; width: 100%; }
                            </style>
                        </head>
                        <body>
                            <div class="card">
                                <h1>
                                    Configuration .env
                                    <button class="btn-add" onclick="addField()">+ Ajouter</button>
                                </h1>
                                <p style="font-size: 0.8rem; color: #94a3b8; margin-bottom: 20px;">Modifiez les variables d'environnement de cette station.</p>
                                <div id="fields"></div>
                                <button class="btn-save" onclick="save()">Sauvegarder et Redémarrer</button>
                                <button class="back-btn" onclick="location.reload()">Annuler</button>
                            </div>
                            <script>
                                let currentConfig = {};
                                
                                try {
                                    currentConfig = JSON.parse('CONFIG_JSON');
                                } catch (e) { console.error(e); }

                                const fields = document.getElementById('fields');
                                
                                function render() {
                                    fields.innerHTML = '';
                                    Object.entries(currentConfig).forEach(([key, val]) => {
                                        const div = document.createElement('div');
                                        div.className = 'form-group';
                                        div.innerHTML = `
                                            <div class="field-col">
                                                <label>CLÉ</label>
                                                <input type="text" class="key-input" value="${key}" onchange="updateKey('${key}', this.value)">
                                            </div>
                                            <div class="field-col">
                                                <label>VALEUR</label>
                                                <input type="text" class="val-input" value="${val}" id="val-${key}">
                                            </div>
                                            <button class="btn-remove" onclick="removeField('${key}')">×</button>
                                        `;
                                        fields.appendChild(div);
                                    });
                                }

                                function updateKey(oldKey, newKey) {
                                    if (oldKey === newKey) return;
                                    const val = currentConfig[oldKey];
                                    delete currentConfig[oldKey];
                                    currentConfig[newKey] = val;
                                    render();
                                }

                                function addField() {
                                    const key = "NOUVELLE_VAR_" + Object.keys(currentConfig).length;
                                    currentConfig[key] = "";
                                    render();
                                }

                                function removeField(key) {
                                    delete currentConfig[key];
                                    render();
                                }

                                function save() {
                                    const updates = {};
                                    document.querySelectorAll('.form-group').forEach(group => {
                                        const key = group.querySelector('.key-input').value;
                                        const val = group.querySelector('.val-input').value;
                                        if (key) updates[key] = val;
                                    });
                                    window.ipc.postMessage('save_settings:' + JSON.stringify(updates));
                                }

                                render();
                            </script>
                        </body>
                        </html>
                    "#;
                    let current_env = load_external_env();
                    let env_json = serde_json::to_string(&current_env).unwrap_or_else(|_| "{}".to_string());
                    let final_html = settings_html.replace("CONFIG_JSON", &env_json);
                    
                    let escaped_html = final_html.replace("`", "\\`").replace("${", "\\${");
                    let script = format!("document.open(); document.write(`{}`); document.close();", escaped_html);
                    let _ = webview.evaluate_script(&script);
                } else {
                    webview.load_url(&url);
                }
            }
Event::UserEvent(UserEvent::FatalError(err)) => {
                let _ = rfd::MessageDialog::new()
                    .set_title("ExeOutput Runtime Error")
                    .set_description(&err)
                    .set_level(rfd::MessageLevel::Error)
                    .show();
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::LoopDestroyed => {
                log("Arrêt de l'application (LoopDestroyed)");
                if let Ok(mut lock) = php_process.lock() {
                    if let Some(mut child) = lock.take() {
                        let _ = child.kill();
                        log("Serveur PHP arrêté.");
                    }
                }
                if let Ok(mut lock) = db_process.lock() {
                    if let Some(mut child) = lock.take() {
                        // Graceful shutdown for MariaDB
                        let exe_dir = env::current_exe().unwrap().parent().unwrap().to_path_buf();
                        let mysql_admin = exe_dir.join("data").join("mysql").join("bin").join("mysqladmin.exe");
                        
                        if mysql_admin.exists() {
                            log("Tentative d'arrêt gracieux de MariaDB...");
                            // We need the port from the config again, but for now we'll try to shut it down
                            // In a real scenario, we'd store the port in a shared state
                            let _ = Command::new(&mysql_admin)
                                .args(&["-u", "root", "shutdown"])
                                .creation_flags(0x08000000)
                                .status();
                            
                            // Give it a moment to stop
                            std::thread::sleep(std::time::Duration::from_millis(500));
                        }
                        
                        let _ = child.kill(); // Ensured kill if admin failed or for other DB types
                        log("Processus de base de données terminé.");
                    }
                }
            }
            _ => (),
        }
    });
}
