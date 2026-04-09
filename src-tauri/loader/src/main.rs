#![windows_subsystem = "windows"]

use std::env;
use std::fs::{self, File};
use std::io::{self, Write, BufRead, BufReader};
use std::process::{Command};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::time::{UNIX_EPOCH, Duration};
use std::sync::{Arc, Mutex};
use std::collections::{HashMap};
use serde::Deserialize;

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

fn log(msg: &str) {
    if let Ok(mut path) = env::current_exe() {
        path.set_extension("log");
        if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), msg);
        }
    }
}

fn main() {
    log("=== Lancement de l'application (Moteur Allégé) ===");
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

    // 1. Extraction des fichiers du projet
    let mut archive = zip::ZipArchive::new(exe_file)
        .map_err(|_| "Aucun contenu de projet trouvé dans cet exécutable. (ZIP manquant)")?;
    
    let temp_dir = env::temp_dir().join(format!("exeoutput_cache_{}_{}_{}", file_name, modified, size));
    log(&format!("Extraction vers {}", temp_dir.display()));
    fs::create_dir_all(&temp_dir)?;
    
    let extraction_marker = temp_dir.join(".extraction_ok");
    
    if !extraction_marker.exists() {
        log("Démarrage de l'extraction...");
        let total_files = archive.len();
        
        // Extraction multithreadée simplifiée
        let num_threads = num_cpus::get().max(1);
        let total_done = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        
        let mut handles = vec![];
        let files_per_thread = (total_files + num_threads - 1) / num_threads;
        
        for t in 0..num_threads {
            let start_idx = t * files_per_thread;
            let end_idx = ((t + 1) * files_per_thread).min(total_files);
            if start_idx >= total_files { break; }
            
            let exe_path_clone = exe_path.clone();
            let temp_dir_clone = temp_dir.clone();
            let total_done_clone = Arc::clone(&total_done);
            
            let handle = std::thread::spawn(move || {
                if let Ok(file) = File::open(&exe_path_clone) {
                    if let Ok(mut thread_archive) = zip::ZipArchive::new(file) {
                        for i in start_idx..end_idx {
                            if let Ok(mut file) = thread_archive.by_index(i) {
                                let outpath = match file.enclosed_name() {
                                    Some(path) => temp_dir_clone.join(path),
                                    None => continue,
                                };
                                if (*file.name()).ends_with('/') {
                                    let _ = fs::create_dir_all(&outpath);
                                } else {
                                    if let Some(p) = outpath.parent() {
                                        let _ = fs::create_dir_all(p);
                                    }
                                    if let Ok(mut outfile) = fs::File::create(&outpath) {
                                        let _ = io::copy(&mut file, &mut outfile);
                                    }
                                }
                            }
                            total_done_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                }
            });
            handles.push(handle);
        }
        
        for handle in handles {
            let _ = handle.join();
        }
        let _ = fs::write(&extraction_marker, "ok");
        log("Extraction terminée.");
    }

    // 2. Chargement de la configuration
    let config_path = temp_dir.join("exeoutput.json");
    if !config_path.exists() {
        return Err("Fichier de configuration introuvable dans le package.".into());
    }
    let config_file = File::open(config_path)?;
    let config: Config = serde_json::from_reader(config_file)?;

    // 3. Junction Bridge (Mapping des dossiers externes)
    let exe_dir = exe_path.parent().unwrap();
    let exe_stem = exe_path.file_stem().map(|s| s.to_str().unwrap_or("data")).unwrap_or("data");
    
    let data_dir = if exe_dir.join("data").is_dir() {
        exe_dir.join("data")
    } else if exe_dir.join(exe_stem).is_dir() {
        exe_dir.join(exe_stem)
    } else {
        exe_dir.join("data")
    };
    
    log(&format!("Dossier de données : {}", data_dir.display()));

    let mut all_external = config.external_dirs.clone().unwrap_or_default();
    if !all_external.contains(&"bootstrap".to_string()) && data_dir.join("bootstrap").is_dir() {
        all_external.push("bootstrap".to_string());
    }

    for dir in &all_external {
        let src = data_dir.join(dir);
        let dst = temp_dir.join(dir);
        if src.is_dir() {
            let _ = Command::new("cmd").args(&["/c", "rmdir", "/s", "/q", dst.to_str().unwrap()]).creation_flags(0x08000000).status();
            let _ = fs::remove_file(&dst); 
            let _ = Command::new("cmd").args(&["/c", "mklink", "/j", dst.to_str().unwrap(), src.to_str().unwrap()]).creation_flags(0x08000000).status();
            log(&format!("Lien : {} -> {}", dir, src.display()));
        }
    }

    // Junction Bridge Inverse (The Bridge)
    if data_dir.exists() {
        if let Ok(entries) = fs::read_dir(&temp_dir) {
            for entry in entries.flatten() {
                if let Ok(meta) = entry.metadata() {
                    if meta.is_dir() {
                        let name = entry.file_name().to_string_lossy().into_owned();
                        if !all_external.contains(&name) && name != "mysql" && !name.starts_with('.') {
                            let bridge_dst = data_dir.join(&name);
                            if !bridge_dst.exists() {
                                let _ = Command::new("cmd").args(&["/c", "mklink", "/j", bridge_dst.to_str().unwrap(), entry.path().to_str().unwrap()]).creation_flags(0x08000000).status();
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Nettoyage du cache bootstrap
    let bootstrap_cache = temp_dir.join("bootstrap").join("cache");
    if bootstrap_cache.exists() {
        let _ = fs::read_dir(&bootstrap_cache).map(|entries| {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        });
    }

    // 5. Lancement des services
    let mut php_process = Command::new("php");
    php_process.arg("-S").arg("127.0.0.1:8080")
               .current_dir(&temp_dir)
               .creation_flags(0x08000000); // Hide console
    
    if let Some(public) = &config.public_dir {
        if !public.is_empty() {
            php_process.arg("-t").arg(public);
        }
    }

    log("Démarrage du serveur PHP sur http://127.0.0.1:8080");
    let mut child = php_process.spawn()?;

    // Ouvrir le navigateur par défaut
    let _ = Command::new("cmd").args(&["/c", "start", "http://127.0.0.1:8080"]).creation_flags(0x08000000).status();

    // Attendre la fin ou un signal
    log("Application lancée. En attente de fermeture...");
    
    // Pour un moteur allégé sans UI, on va juste attendre que le processus PHP s'arrête
    // Ou on pourrait mettre un message d'information
    let _ = child.wait()?;
    log("Serveur PHP arrêté. Fin du programme.");

    Ok(())
}
