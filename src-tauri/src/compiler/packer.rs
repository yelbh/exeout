use std::path::{Path, PathBuf};
use std::fs::{self, File};
use std::io::{BufReader};
use anyhow::Result;
use zip::write::FileOptions;

/// Directories and files to skip during packaging (these would cause huge,
/// slow, or broken executables).
const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".github",
    "tests",
    ".svn",
    ".hg",
    "__pycache__",
    ".idea",
    ".vscode",
    "storage/framework",
    "storage/logs",
    "storage/debugbar",
    "bootstrap/cache",
];

const IGNORED_EXTENSIONS: &[&str] = &[
    "exe", "dll", "pdb", "obj", "lib",
];

pub struct Compiler {
    pub source_dir: PathBuf,
    pub output_path: PathBuf,
    pub version: String,
    pub icon_path: Option<PathBuf>,
    pub entry_point: String,
    pub public_dir: String,
    pub external_dirs: Vec<String>,
    pub db_type: String,
    pub db_port: u32,
    pub db_name: String,
    pub db_user: String,
    pub db_pass: String,
    pub init_sql_path: Option<PathBuf>,
    pub update_url: Option<String>,
    pub notes: Option<String>,
    pub env_vars: std::collections::HashMap<String, String>,
    pub php_extensions: Vec<String>,
    pub php_portable_path: Option<PathBuf>,
}

impl Compiler {
    pub fn new(source: &str, output: &str, entry_point: &str, public_dir: &str) -> Self {
        Self {
            source_dir: PathBuf::from(source),
            output_path: PathBuf::from(output),
            version: "1.0.0".to_string(),
            icon_path: None,
            entry_point: entry_point.to_string(),
            public_dir: public_dir.to_string(),
            external_dirs: Vec::new(),
            db_type: "none".to_string(),
            db_port: 3307,
            db_name: "".to_string(),
            db_user: "root".to_string(),
            db_pass: "".to_string(),
            init_sql_path: None,
            update_url: None,
            notes: None,
            env_vars: std::collections::HashMap::new(),
            php_extensions: Vec::new(),
            php_portable_path: None,
        }
    }

    pub fn generate_update_manifest(&self) -> Result<()> {
        if let Some(url) = &self.update_url {
            let filename = url.split('/').last().unwrap_or("updater.json");
            if !filename.ends_with(".json") {
                return Ok(());
            }

            let mut manifest_path = self.output_path.clone();
            manifest_path.set_file_name(filename);

            let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
            let notes = self.notes.as_deref().unwrap_or("Mise à jour générée par ExeOutput Studio.");

            let manifest = serde_json::json!({
                "version": self.version,
                "notes": notes,
                "pub_date": now,
                "platforms": {
                    "windows-x86_64": {
                        "signature": "...",
                        "url": url.replace(".json", ".msi.zip")
                    }
                }
            });

            let content = serde_json::to_string_pretty(&manifest)?;
            fs::write(manifest_path, content)?;
        }
        Ok(())
    }

    pub fn collect_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        self.collect_recursive(&self.source_dir, &mut files)?;
        Ok(files)
    }

    fn collect_recursive(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    // Skip ignored directories
                    let dir_name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    let rel_path_str = get_relative_path(&self.source_dir, &path)
                        .unwrap_or_else(|| dir_name.to_string());
                    
                    let dir_name_lower = dir_name.to_lowercase();
                    let rel_path_str_lower = rel_path_str.to_lowercase();

                    if IGNORED_DIRS.contains(&dir_name_lower.as_str()) || IGNORED_DIRS.contains(&rel_path_str_lower.as_str()) {
                        continue;
                    }
                    
                    // Skip external directories from the ZIP collection (but NEVER skip bootstrap)
                    if self.external_dirs.contains(&dir_name.to_string()) && dir_name != "bootstrap" {
                        continue;
                    }
                    
                    self.collect_recursive(&path, files)?;
                } else {
                    // Skip ignored file extensions
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    if IGNORED_EXTENSIONS.contains(&ext.as_str()) {
                        continue;
                    }
                    files.push(path);
                }
            }
        }
        Ok(())
    }

    /// Stream files into a ZIP archive and report progress on every file.
    /// Uses `BufReader` to avoid loading entire files into RAM.
    pub fn compress_resources<F>(&self, files: Vec<PathBuf>, progress: F) -> Result<Vec<u8>>
    where
        F: Fn(u32),
    {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let options = FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .unix_permissions(0o755);

            let total = files.len() as u32;

            for (i, file) in files.iter().enumerate() {
                // Ensure we have a relative path from the source directory
                let zip_path = get_relative_path(&self.source_dir, file)
                    .unwrap_or_else(|| {
                        let rel_path = file.strip_prefix(&self.source_dir).unwrap_or(file);
                        rel_path.to_string_lossy().replace('\\', "/")
                    });
                
                // Remove driving letters or leading slashes if any (safety check)
                let zip_path = zip_path.trim_start_matches(|c: char| c == '/' || c.is_ascii_alphabetic() && zip_path.get(1..2) == Some(":"));
                let zip_path = if zip_path.starts_with(':') { &zip_path[1..] } else { &zip_path };
                let zip_path = zip_path.trim_start_matches('/');

                zip.start_file(zip_path, options)?;

                // If it is a PHP file, minify it by stripping comments and unneeded spaces.
                // Otherwise, stream it to avoid memory issues.
                let is_php = file
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.to_lowercase() == "php")
                    .unwrap_or(false);

                if is_php {
                    if let Ok(content) = fs::read_to_string(file) {
                        let minified = minify_php(&content);
                        let mut cursor = std::io::Cursor::new(minified.into_bytes());
                        std::io::copy(&mut cursor, &mut zip)?;
                    } else {
                        let f = File::open(file)?;
                        let mut reader = BufReader::with_capacity(64 * 1024, f);
                        std::io::copy(&mut reader, &mut zip)?;
                    }
                } else {
                    let f = File::open(file)?;
                    let mut reader = BufReader::with_capacity(64 * 1024, f);
                    std::io::copy(&mut reader, &mut zip)?;
                }

                // Report progress on every file
                let i_u32 = (i + 1) as u32;
                if total > 0 {
                    progress(i_u32 * 100 / total);
                }
            }

            if total == 0 {
                progress(100);
            }

            // 3. Add configuration file
            let config = serde_json::json!({
                "version": self.version,
                "entry_point": self.entry_point,
                "public_dir": self.public_dir,
                "external_dirs": self.external_dirs,
                "db_type": self.db_type,
                "db_port": self.db_port,
                "db_name": self.db_name,
                "db_user": self.db_user,
                "db_pass": self.db_pass,
                "has_init_sql": self.init_sql_path.is_some(),
                "update_url": self.update_url,
                "php_extensions": self.php_extensions
            });
            let config_json = serde_json::to_vec_pretty(&config)?;
            zip.start_file("exeoutput.json", options)?;
            std::io::copy(&mut std::io::Cursor::new(config_json), &mut zip)?;

            zip.finish()?;
        }
        Ok(buf.into_inner())
    }

    pub fn generate_exe(&self, compressed: Vec<u8>) -> Result<()> {
        let exe_name = self.output_path.clone();
        if let Some(parent) = exe_name.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read the native Windows loader
        let loader_bytes = include_bytes!("../../resources/loader.exe");

        // Write the base loader to the output path FIRST
        fs::write(&self.output_path, loader_bytes)?;

        // If an icon is selected, update the PE resources NOW.
        // WARNING: Windows EndUpdateResourceW restructures the PE executable
        // and completely destroys any appended overlay data. We MUST do this
        // before appending our ZIP payload!
        if let Some(icon) = &self.icon_path {
            self.apply_icon(icon)?;
        }

        // Encrypt the compressed ZIP archive using AES-256-GCM
        const ENCRYPTION_KEY: &[u8; 32] = b"ex30utput_pr0tect_key_2026_aes22";
        let encrypted_payload = crate::utils::crypto::encrypt_file(&compressed, ENCRYPTION_KEY)
            .map_err(|e| anyhow::anyhow!("Encryption error: {}", e))?;

        // FINALLY, append the encrypted ZIP payload as overlay data
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(&self.output_path)?;
        file.write_all(&encrypted_payload)?;

        // Write the length of the encrypted payload as a little-endian u64 (8 bytes) at the very end
        let len = encrypted_payload.len() as u64;
        file.write_all(&len.to_le_bytes())?;

        // Ensure data directory exists if we have external dirs or a database
        if !self.external_dirs.is_empty() || self.db_type != "none" || self.init_sql_path.is_some() {
            let data_dir = exe_name.parent().unwrap().join("data");
            fs::create_dir_all(&data_dir)?;
            
            // Copy explicit external directories
            for ext_dir in &self.external_dirs {
                if ext_dir == "bootstrap" { continue; }
                let src_dir = self.source_dir.join(ext_dir);
                if src_dir.exists() {
                    let dest_dir = data_dir.join(ext_dir);
                    self.copy_dir_recursive(&src_dir, &dest_dir)?;
                }
            }

            // If MariaDB is selected but not in external_dirs, try to find it
            if self.db_type == "mariadb" && !self.external_dirs.contains(&"mysql".to_string()) {
                let mysql_src = self.source_dir.join("mysql");
                if mysql_src.exists() {
                    let mysql_dest = data_dir.join("mysql");
                    self.copy_dir_recursive(&mysql_src, &mysql_dest)?;
                }
            }

            // Handle bootstrap cache externalization (part of fixing path poisoning)
            let bootstrap_cache_dest = data_dir.join("bootstrap").join("cache");
            let _ = fs::create_dir_all(&bootstrap_cache_dest);
        }

        if let Some(sql_path) = &self.init_sql_path {
            let data_dir = exe_name.parent().unwrap().join("data");
            let dest_sql = data_dir.join("init.sql");
            if sql_path.exists() {
                fs::copy(sql_path, dest_sql)?;
            }
        }

        // Generate template .env for multi-station deployment
        if !self.env_vars.is_empty() || self.db_type == "mariadb" {
            let env_path = exe_name.parent().unwrap().join(".env.template");
            let mut content = String::new();
            content.push_str("# ==================================================\n");
            content.push_str("# MODELE DE CONFIGURATION (A RENOMMER EN .env)\n");
            content.push_str("# ==================================================\n\n");
            
            let mut vars = self.env_vars.clone();
            
            // Add DB vars if MariaDB is used and not already in env_vars
            if self.db_type == "mariadb" {
                vars.entry("DB_CONNECTION".to_string()).or_insert("mysql".to_string());
                vars.entry("DB_HOST".to_string()).or_insert("127.0.0.1".to_string());
                vars.entry("DB_PORT".to_string()).or_insert(self.db_port.to_string());
                vars.entry("DB_DATABASE".to_string()).or_insert(self.db_name.clone());
                vars.entry("DB_USERNAME".to_string()).or_insert(self.db_user.clone());
                vars.entry("DB_PASSWORD".to_string()).or_insert(self.db_pass.clone());
            }

            for (key, val) in &vars {
                content.push_str(&format!("{}={}\n", key, val));
            }
            
            content.push_str("\n# Note: Vous pouvez aussi modifier ces valeurs via Ctrl+Shift+S dans l'application.\n");
            fs::write(env_path, content)?;
        }

        // Auto-copier le dossier PHP portable
        // Priorité 1 : Chemin PHP configuré explicitement dans le projet
        // Priorité 2 : Dossier php/ dans le dossier source
        let php_dest = exe_name.parent().unwrap().join("php");
        if let Some(ref php_configured) = self.php_portable_path {
            if php_configured.is_dir() && !php_dest.exists() {
                let _ = self.copy_dir_recursive(php_configured, &php_dest);
            } else if php_configured.is_dir() && php_dest.exists() {
                // Toujours remplacer si configuré explicitement
                let _ = fs::remove_dir_all(&php_dest);
                let _ = self.copy_dir_recursive(php_configured, &php_dest);
            }
        } else {
            // Fallback : chercher dans le dossier source
            let php_src = self.source_dir.join("php");
            if php_src.is_dir() && !php_dest.exists() {
                let _ = self.copy_dir_recursive(&php_src, &php_dest);
            }
        }

        Ok(())
    }

    fn copy_dir_recursive(&self, src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_path = dst.join(entry.file_name());
            if path.is_dir() {
                self.copy_dir_recursive(&path, &dest_path)?;
            } else {
                fs::copy(&path, &dest_path)?;
            }
        }
        Ok(())
    }

    pub fn embed_runtime(&self) -> Result<()> {
        Ok(())
    }

    pub fn apply_icon(&self, icon_path: &Path) -> Result<()> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use winapi::um::winbase::{BeginUpdateResourceW, UpdateResourceW, EndUpdateResourceW};
            use winapi::um::winuser::{RT_ICON, RT_GROUP_ICON};
            use winapi::um::winnt::{MAKELANGID, LANG_NEUTRAL, SUBLANG_NEUTRAL};
            use winapi::um::errhandlingapi::GetLastError;
            use winapi::shared::minwindef::{FALSE, TRUE, LPVOID};

            let exe_path_wide: Vec<u16> = self.output_path.as_os_str().encode_wide().chain(Some(0)).collect();
            let icon_data = fs::read(icon_path)?;

            if icon_data.len() < 6 {
                return Err(anyhow::anyhow!("Fichier icône invalide (trop court)"));
            }

            // Vérifier la signature ICO (idReserved=0, idType=1)
            let id_reserved = u16::from_le_bytes([icon_data[0], icon_data[1]]);
            let id_type     = u16::from_le_bytes([icon_data[2], icon_data[3]]);
            if id_reserved != 0 || id_type != 1 {
                return Err(anyhow::anyhow!("Format ICO invalide (signature incorrecte)"));
            }

            let count = u16::from_le_bytes([icon_data[4], icon_data[5]]) as usize;
            if count == 0 {
                return Err(anyhow::anyhow!("Fichier ICO vide (aucune image)"));
            }

            // ICONDIRENTRY (fichier .ico) = 16 octets
            // GRPICONDIRENTRY (ressource PE) = les 12 premiers octets + WORD nID (14 octets total)
            let mut icon_images: Vec<Vec<u8>> = Vec::new();
            let mut group_icon_data: Vec<u8> = icon_data[0..6].to_vec();

            for i in 0..count {
                let entry_start = 6 + i * 16;
                if entry_start + 16 > icon_data.len() { break; }
                let entry = &icon_data[entry_start..entry_start + 16];

                let img_size   = u32::from_le_bytes([entry[8],  entry[9],  entry[10], entry[11]]) as usize;
                let img_offset = u32::from_le_bytes([entry[12], entry[13], entry[14], entry[15]]) as usize;

                if img_offset == 0 || img_offset + img_size > icon_data.len() { continue; }

                icon_images.push(icon_data[img_offset..img_offset + img_size].to_vec());

                // GRPICONDIRENTRY : 12 octets depuis ICONDIRENTRY + WORD nID
                group_icon_data.extend_from_slice(&entry[0..12]);
                let icon_id: u16 = (i as u16) + 1; // ID des images RT_ICON (1, 2, 3...)
                group_icon_data.extend_from_slice(&icon_id.to_le_bytes());
            }

            if icon_images.is_empty() {
                return Err(anyhow::anyhow!("Aucune image valide trouvée dans le fichier ICO"));
            }

            let lang = MAKELANGID(LANG_NEUTRAL, SUBLANG_NEUTRAL);

            unsafe {
                let handle = BeginUpdateResourceW(exe_path_wide.as_ptr(), TRUE);
                if handle.is_null() {
                    let err = GetLastError();
                    return Err(anyhow::anyhow!("BeginUpdateResourceW échoué (erreur Windows : {})", err));
                }

                // Mettre à jour le RT_GROUP_ICON (ID=1)
                let ok = UpdateResourceW(
                    handle,
                    RT_GROUP_ICON,
                    128usize as *const u16, // ID du groupe (128 est le standard Tauri/Windows)
                    lang,
                    group_icon_data.as_ptr() as LPVOID,
                    group_icon_data.len() as u32,
                );
                if ok == 0 {
                    let err = GetLastError();
                    EndUpdateResourceW(handle, TRUE); // annuler
                    return Err(anyhow::anyhow!("UpdateResourceW RT_GROUP_ICON échoué (erreur : {})", err));
                }

                // Mettre à jour chaque RT_ICON
                for (i, img) in icon_images.iter().enumerate() {
                    UpdateResourceW(
                        handle,
                        RT_ICON,
                        (i + 1) as *const u16,
                        lang,
                        img.as_ptr() as LPVOID,
                        img.len() as u32,
                    );
                }

                // Valider (FALSE = appliquer les changements)
                if EndUpdateResourceW(handle, FALSE) == 0 {
                    let err = GetLastError();
                    return Err(anyhow::anyhow!("EndUpdateResourceW échoué (erreur : {}) — icône non sauvegardée", err));
                }
            }
            println!("Icône appliquée ({} image(s)) : {}", icon_images.len(), self.output_path.display());
        }
        Ok(())
    }
}

fn minify_php(content: &str) -> String {
    let mut result = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_single_comment = false;
    let mut in_multi_comment = false;
    let mut in_string = false;
    let mut string_char = ' ';

    while let Some(c) = chars.next() {
        if in_single_comment {
            if c == '\n' || c == '\r' {
                in_single_comment = false;
                result.push(c);
            }
            continue;
        }
        if in_multi_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_multi_comment = false;
            }
            continue;
        }
        if in_string {
            if c == '\\' {
                result.push(c);
                if let Some(next_c) = chars.next() {
                    result.push(next_c);
                }
                continue;
            }
            result.push(c);
            if c == string_char {
                in_string = false;
            }
            continue;
        }

        // Check for comment starts
        if c == '/' && chars.peek() == Some(&'/') {
            chars.next();
            in_single_comment = true;
            continue;
        }
        if c == '#' {
            in_single_comment = true;
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_multi_comment = true;
            continue;
        }

        // Check for string starts
        if c == '"' || c == '\'' {
            in_string = true;
            string_char = c;
            result.push(c);
            continue;
        }

        result.push(c);
    }
    result
}

fn get_relative_path(root: &Path, path: &Path) -> Option<String> {
    let root_str = root.to_string_lossy().replace('\\', "/").to_lowercase();
    let path_str = path.to_string_lossy().replace('\\', "/").to_lowercase();
    
    let root_prefix = if root_str.ends_with('/') { root_str.clone() } else { format!("{}/", root_str) };
    
    if path_str.starts_with(&root_prefix) {
        let start_idx = root_prefix.len();
        let orig_path_str = path.to_string_lossy().replace('\\', "/");
        if start_idx <= orig_path_str.len() {
            return Some(orig_path_str[start_idx..].to_string());
        }
    } else if path_str == root_str {
        return Some(String::new());
    }
    
    None
}

