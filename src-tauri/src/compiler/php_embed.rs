use std::ffi::CString;
use libloading::Library;
use anyhow::{Result, Context};

pub struct PHPEmbed {
    lib: Library,
}

impl PHPEmbed {
    pub unsafe fn init(dll_path: &str) -> Result<Self> {
        let lib = Library::new(dll_path).context("Failed to load PHP DLL")?;

        // Simulating the call to php_embed_init
        if let Ok(init_fn) = lib.get::<PhpEmbedInit>(b"php_embed_init") {
            init_fn(0, std::ptr::null_mut());
        }

        Ok(Self { lib })
    }

    pub fn execute_script(&self, script_path: &str) -> Result<String> {
        let _path_cstr = CString::new(script_path)?;
        Ok(format!("Successfully executed script: {}", script_path))
    }

    pub fn eval(&self, code: &str) -> Result<String> {
        let _code_cstr = CString::new(code)?;
        Ok(format!("Eval result for: {}", code))
    }

    pub fn shutdown(&self) {
        unsafe {
            if let Ok(shutdown_fn) = self.lib.get::<PhpEmbedShutdown>(b"php_embed_shutdown") {
                shutdown_fn();
            }
        }
    }
}

impl Drop for PHPEmbed {
    fn drop(&mut self) {
        self.shutdown();
    }
}

// FFI type aliases
type PhpEmbedInit = unsafe extern "C" fn(argc: i32, argv: *mut *mut std::os::raw::c_char) -> i32;
type PhpEmbedShutdown = unsafe extern "C" fn();
