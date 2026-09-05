use std::path::PathBuf;

pub fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    let candidates = [std::env::var_os("USERPROFILE"), std::env::var_os("HOME")];
    #[cfg(not(windows))]
    let candidates = [std::env::var_os("HOME")];
    candidates
        .into_iter()
        .flatten()
        .map(PathBuf::from)
        .find(|path| path.is_absolute())
}

// Desktop and its MCP subprocess must open the same index even when HOME is
// absent (normal on Windows). Never silently create an index in the working tree.
pub fn metadata_dir() -> Result<PathBuf, String> {
    home_dir()
        .map(|home| home.join("cowiki").join(".cowiki"))
        .ok_or_else(|| {
            "Cannot locate your home directory; set USERPROFILE (Windows) or HOME".to_string()
        })
}
