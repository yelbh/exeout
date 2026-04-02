use std::path::{Path, PathBuf};
use anyhow::Result;
use std::fs;

// EmbeddedAssets is only active when the folder exists.
// Using a manual approach to avoid compile failures on missing folder.
pub struct ResourceManager {
    pub base_path: PathBuf,
}

impl ResourceManager {
    pub fn new(base: &str) -> Self {
        Self {
            base_path: PathBuf::from(base),
        }
    }

    pub fn add_file(&self, path: &Path, content: &[u8]) -> Result<()> {
        let full_path = self.base_path.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full_path, content)?;
        Ok(())
    }

    pub fn add_directory(&self, path: &Path) -> Result<()> {
        let full_path = self.base_path.join(path);
        fs::create_dir_all(full_path)?;
        Ok(())
    }

    pub fn extract_at_runtime(&self, target_dir: &Path) -> Result<()> {
        // Placeholder: In the compiled app, embedded assets would be
        // extracted here at startup. Requires rust-embed with a valid folder.
        let _ = target_dir;
        Ok(())
    }
}
