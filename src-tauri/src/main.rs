#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod compiler {
    pub mod php_embed;
    pub mod packer;
    pub mod resources;
}
mod server {
    pub mod http;
}
mod utils {
    pub mod crypto;
    pub mod compression;
    pub mod windows;
}
mod runtime {
    pub mod generator;
}

use std::sync::Arc;
use tokio::sync::Mutex;
use crate::compiler::php_embed::PHPEmbed;

struct AppState {
    php: Arc<Mutex<Option<Arc<PHPEmbed>>>>,
    db_process: Arc<Mutex<Option<std::process::Child>>>,
    preview_process: Arc<Mutex<Option<std::process::Child>>>,
}

#[derive(serde::Deserialize)]
struct ServerSettings {
    host: String,
    user: String,
    pass: String,
    port: u16,
    #[serde(rename = "remotePath")]
    remote_path: String,
}

#[derive(serde::Deserialize)]
struct DatabaseConfig {
    #[serde(rename = "type")]
    db_type: String, // "none", "sqlite", "mariadb"
    port: Option<u32>,
    #[serde(rename = "databaseName")]
    database_name: Option<String>,
    username: Option<String>,
    password: Option<String>,
    #[serde(rename = "initSqlPath")]
    init_sql_path: Option<String>,
}

#[tauri::command]
async fn deploy_project(window: tauri::Window, exe_path: String, json_path: String, server: ServerSettings) -> Result<String, String> {
    use std::net::TcpStream;
    use ssh2::Session;
    use std::io::Read;

    tokio::task::spawn_blocking(move || {
        let _ = window.emit("compilation-log", format!("Connexion au serveur {}...", server.host));
        
        let tcp = TcpStream::connect(format!("{}:{}", server.host, server.port))
            .map_err(|e| format!("Erreur de connexion TCP : {}", e))?;
        
        let mut sess = Session::new().map_err(|e| format!("Erreur session SSH : {}", e))?;
        sess.set_tcp_stream(tcp);
        sess.handshake().map_err(|e| format!("Échec du handshake SSH : {}", e))?;
        
        sess.userauth_password(&server.user, &server.pass)
            .map_err(|e| format!("Échec d'authentification : {}", e))?;
        
        if !sess.authenticated() {
            return Err("Accès refusé par le serveur".to_string());
        }

        let sftp = sess.sftp().map_err(|e| format!("Erreur SFTP : {}", e))?;
        
        // 1. Envoyer le .exe
        let exe_file_path = std::path::Path::new(&exe_path);
        let exe_name = exe_file_path.file_name().unwrap().to_str().unwrap();
        let remote_exe = std::path::Path::new(&server.remote_path).join(exe_name);
        
        let _ = window.emit("compilation-log", format!("Envoi de {}...", exe_name));
        let mut local_exe = std::fs::File::open(exe_file_path).map_err(|e| e.to_string())?;
        let mut remote_exe_file = sftp.create(&remote_exe).map_err(|e| e.to_string())?;
        std::io::copy(&mut local_exe, &mut remote_exe_file).map_err(|e| e.to_string())?;

        // 2. Envoyer le .json
        let json_file_path = std::path::Path::new(&json_path);
        let json_name = json_file_path.file_name().unwrap().to_str().unwrap();
        let remote_json = std::path::Path::new(&server.remote_path).join(json_name);
        
        let _ = window.emit("compilation-log", format!("Envoi de {}...", json_name));
        let mut local_json = std::fs::File::open(json_file_path).map_err(|e| e.to_string())?;
        let mut remote_json_file = sftp.create(&remote_json).map_err(|e| e.to_string())?;
        std::io::copy(&mut local_json, &mut remote_json_file).map_err(|e| e.to_string())?;

        Ok("Déploiement réussi sur votre serveur !".to_string())
    }).await.map_err(|e| format!("Erreur thread: {}", e))?
}

#[tauri::command]
async fn compile_project(window: tauri::Window, _name: String, version: String, source: String, output: String, entry_point: String, public_dir: String, external_dirs: Vec<String>, icon_path: Option<String>, database_config: Option<DatabaseConfig>, update_url: Option<String>, notes: Option<String>) -> Result<String, String> {
    let out_path = std::path::Path::new(&output).to_path_buf();
    let out_str = output.clone();
    
    tokio::task::spawn_blocking(move || {
        use crate::compiler::packer::Compiler;
        let mut compiler = Compiler::new(&source, &out_str, &entry_point, &public_dir);
        compiler.version = version;
        compiler.external_dirs = external_dirs;
        compiler.notes = notes;
        
        if let Some(url) = update_url {
            compiler.update_url = Some(url);
        }
        
        if let Some(db) = database_config {
            compiler.db_type = db.db_type;
            compiler.db_port = db.port.unwrap_or(3307);
            compiler.db_name = db.database_name.unwrap_or_default();
            compiler.db_user = db.username.unwrap_or_else(|| "root".to_string());
            compiler.db_pass = db.password.unwrap_or_default();
            
            if let Some(sql_path) = db.init_sql_path {
                if !sql_path.is_empty() {
                    compiler.init_sql_path = Some(std::path::PathBuf::from(sql_path));
                }
            }
        }
        
        if let Some(icon) = icon_path {
            compiler.icon_path = Some(std::path::PathBuf::from(icon));
        }
        
        let _ = window.emit("compilation-progress", 0);
        
        // 1. Collect files (25%)
        let files = compiler.collect_files()
            .map_err(|e| format!("Erreur lors de la collecte : {}", e))?;
        let file_count = files.len();
        let _ = window.emit("compilation-log", format!("{} fichier(s) trouvé(s) à packager", file_count));
        let _ = window.emit("compilation-progress", 25);
        
        // 2. Package resources (50%)
        let compressed = compiler.compress_resources(files, |pct| {
            let overall = 25 + (pct * 50 / 100);
            let _ = window.emit("compilation-progress", overall);
        }).map_err(|e| format!("Erreur lors de la compression : {}", e))?;
        
        // 3. Generate the final EXE (90%)
        compiler.generate_exe(compressed)
            .map_err(|e| format!("Erreur lors de la génération de l'EXE : {}", e))?;
        let _ = window.emit("compilation-progress", 90);

        // 4. Generate Update Manifest (100%)
        let _ = window.emit("compilation-log", "Génération du manifeste de mise à jour...");
        compiler.generate_update_manifest()
            .map_err(|e| format!("Erreur lors de la génération du manifeste : {}", e))?;
            
        let _ = window.emit("compilation-progress", 100);
        
        Ok(format!("Compilation terminée ! Votre exécutable et son manifeste JSON sont disponibles ici : {}", out_path.parent().unwrap().display()))
    }).await.map_err(|e| format!("Erreur du thread: {}", e))?
}

#[tauri::command]
async fn preview_project(state: tauri::State<'_, AppState>, source: String, database_config: Option<DatabaseConfig>) -> Result<u16, String> {
    use std::process::Command;
    use std::net::TcpListener;

    // 1. Kill existing DB and Preview processes
    {
        let mut db_lock = state.db_process.lock().await;
        if let Some(mut child) = db_lock.take() {
            let _ = child.kill();
        }

        let mut preview_lock = state.preview_process.lock().await;
        if let Some(mut child) = preview_lock.take() {
            let _ = child.kill();
        }
    }

    // 2. Start MariaDB if requested
    let mut db_env_port = 3307;
    let mut db_name = String::new();
    if let Some(db) = &database_config {
        if db.db_type == "mariadb" {
            db_env_port = db.port.unwrap_or(3307);
            db_name = db.database_name.clone().unwrap_or_default();
            let source_path = std::path::Path::new(&source);
            let mysql_dir = source_path.join("mysql");
            let mysqld_exe = mysql_dir.join("bin").join("mysqld.exe");
            
            if mysqld_exe.exists() {
                let data_dir = mysql_dir.join("data");
                if !data_dir.exists() {
                    let _ = std::fs::create_dir_all(&data_dir);
                }

                let mut db_cmd = Command::new(&mysqld_exe);
                db_cmd.arg("--no-defaults")
                      .arg(format!("--datadir={}", data_dir.to_str().unwrap()))
                      .arg(format!("--port={}", db_env_port))
                      .arg("--bind-address=127.0.0.1")
                      .arg("--skip-grant-tables")
                      .arg("--console");

                #[cfg(windows)]
                {
                    use std::os::windows::process::CommandExt;
                    db_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
                }

                match db_cmd.spawn() {
                    Ok(child) => {
                         let mut lock = state.db_process.lock().await;
                         *lock = Some(child);
                         
                         // Wait for MariaDB to be ready (max 10s)
                         let mut connected = false;
                         for _ in 0..100 {
                             if std::net::TcpStream::connect(("127.0.0.1", db_env_port as u16)).is_ok() {
                                 connected = true;
                                 break;
                             }
                             std::thread::sleep(std::time::Duration::from_millis(100));
                         }

                         if connected && !db_name.is_empty() {
                            let mysql_cli = mysql_dir.join("bin").join("mysql.exe");
                            if mysql_cli.exists() {
                                // Create DB
                                let mut create_cmd = Command::new(&mysql_cli);
                                create_cmd.args(&["-u", "root", &format!("-P{}", db_env_port), "-e", &format!("CREATE DATABASE IF NOT EXISTS `{}`;", db_name)]);
                                
                                #[cfg(windows)]
                                {
                                    use std::os::windows::process::CommandExt;
                                    create_cmd.creation_flags(0x08000000);
                                }
                                
                                let _ = create_cmd.status();

                                // Import init.sql if provided
                                if let Some(sql_path) = &db.init_sql_path {
                                    if !sql_path.is_empty() {
                                        if let Ok(file) = std::fs::File::open(sql_path) {
                                            let mut import_cmd = Command::new(&mysql_cli);
                                            import_cmd.args(&["-u", "root", &format!("-P{}", db_env_port), &db_name])
                                                      .stdin(std::process::Stdio::from(file));
                                            
                                            #[cfg(windows)]
                                            {
                                                use std::os::windows::process::CommandExt;
                                                import_cmd.creation_flags(0x08000000);
                                            }
                                            
                                            let _ = import_cmd.status();
                                        }
                                    }
                                }
                            }
                         }
                    },
                    Err(e) => return Err(format!("Erreur lors du lancement de MariaDB : {}", e)),
                }
            }
        }
    }

    // 3. Find a free port for PHP
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    drop(listener); // Release the port so PHP can use it

    // 4. Start PHP Server
    let source_path = std::path::Path::new(&source);
    
    // Determine doc root: prioritize source/public if it exists (Laravel style)
    let mut doc_root = source_path.to_path_buf();
    let mut _using_public_subfolder = false;
    if source_path.join("public").is_dir() {
        doc_root = source_path.join("public");
        _using_public_subfolder = true;
    }

    // Prepare session path (common fix for login loops in Laravel + php -S)
    let session_path = source_path.join("storage").join("framework").join("sessions");
    let _ = std::fs::create_dir_all(&session_path);
    let session_path_str = session_path.to_str().unwrap().to_string();

    let mut php_cmd = Command::new("php");
    php_cmd.arg("-S").arg(format!("127.0.0.1:{}", port))
           .arg("-t").arg(doc_root);
           
    // Laravel/Livewire fix: the PHP built-in server returns 404 for URLs with dots (like livewire.js)
    // unless we explicitly pass server.php as the router script.
    let server_php = source_path.join("server.php");
    if server_php.exists() {
        php_cmd.arg(server_php);
    }

    php_cmd.arg("-d").arg(format!("session.save_path={}", session_path_str))
           .arg("-d").arg("session.gc_probability=0") // Avoid permission issues with GC
           .arg("-d").arg("display_errors=1")
           .arg("-d").arg("error_reporting=E_ALL")
           .current_dir(source_path);

    // Pass DB and Session credentials via ENV (common for Laravel/modern apps)
    // We use the 'file' driver for the preview. Filament sessions are too large for the 'cookie' driver.
    php_cmd.env("SESSION_DRIVER", "file"); 
    php_cmd.env("SESSION_SECURE_COOKIE", "false");
    php_cmd.env("SESSION_DOMAIN", "null"); 
    php_cmd.env("SESSION_SAME_SITE", "lax");
    php_cmd.env("SESSION_HTTP_ONLY", "true");
    php_cmd.env("SESSION_LIFETIME", "120");
    php_cmd.env("APP_DEBUG", "true");
    php_cmd.env("APP_ENV", "local");
    php_cmd.env("APP_URL", format!("http://127.0.0.1:{}", port));

    if let Some(db) = &database_config {
        if db.db_type != "none" {
            php_cmd.env("DB_HOST", "127.0.0.1");
            php_cmd.env("DB_PORT", db_env_port.to_string());
            php_cmd.env("DB_DATABASE", db_name);
            php_cmd.env("DB_USERNAME", db.username.as_deref().unwrap_or_default());
            php_cmd.env("DB_PASSWORD", db.password.as_deref().unwrap_or_default());
        }
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        php_cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    match php_cmd.spawn() {
        Ok(child) => {
            let mut lock = state.preview_process.lock().await;
            *lock = Some(child);
            Ok(port)
        },
        Err(e) => Err(format!("Impossible de lancer le serveur PHP d'aperçu.\nAssurez-vous que PHP est installé et dans votre PATH.\nErreur: {}", e))
    }
}

#[tauri::command]
async fn save_config(path: String, config: serde_json::Value) -> Result<(), String> {
    let content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Erreur de sérialisation : {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| format!("Erreur d'écriture : {}", e))?;
    Ok(())
}

#[tauri::command]
fn get_project_dirs(path: String) -> Result<Vec<String>, String> {
    let mut dirs = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        if !name.starts_with('.') && name != "node_modules" {
                            dirs.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    dirs.sort();
    Ok(dirs)
}

#[tauri::command]
async fn load_config(path: String) -> Result<serde_json::Value, String> {
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Erreur de lecture : {}", e))?;
    let config: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Erreur de parsing JSON : {}", e))?;
    Ok(config)
}

#[tauri::command]
async fn get_php_versions() -> Result<Vec<String>, String> {
    Ok(vec!["8.1".to_string(), "8.2".to_string(), "8.3".to_string()])
}

#[tauri::command]
async fn get_file_list(_path: String) -> Result<Vec<String>, String> {
    Ok(vec![])
}

#[tauri::command]
async fn validate_php_syntax(_code: String) -> Result<bool, String> {
    Ok(true)
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            php: Arc::new(Mutex::new(None)),
            db_process: Arc::new(Mutex::new(None)),
            preview_process: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            compile_project,
            deploy_project,
            preview_project,
            save_config,
            load_config,
            get_project_dirs,
            get_php_versions,
            get_file_list,
            validate_php_syntax
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
