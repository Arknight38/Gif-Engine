use std::path::PathBuf;

pub fn config_dir() -> Option<PathBuf> {
    // On Windows, use %APPDATA%
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    }

    // On other platforms, fallback to simple home or local (though this app is Windows-focused)
    #[cfg(not(target_os = "windows"))]
    {
        // Simple fallback to current directory for non-windows if not using `dirs` crate
        Some(PathBuf::from("."))
    }
}


