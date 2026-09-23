use eframe::egui::{FontData, FontDefinitions, FontFamily};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

include!(concat!(env!("OUT_DIR"), "/examples.rs"));

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: String,
    pub language: String,
    pub reduced_motion: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: "light".into(),
            language: "en".into(),
            reduced_motion: false,
        }
    }
}
impl Preferences {
    pub fn load(root: &Path) -> Self {
        let mut prefs: Self = fs::read(root.join("preferences.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        if !["light", "dark"].contains(&prefs.theme.as_str()) {
            prefs.theme = "light".into();
        }
        if !["en", "zh", "ja"].contains(&prefs.language.as_str()) {
            prefs.language = "en".into();
        }
        prefs
    }
    pub fn save(&self, root: &Path) -> Result<(), String> {
        let destination = root.join("preferences.json");
        let temp = root.join("preferences.json.tmp");
        fs::write(&temp, serde_json::to_vec_pretty(self).unwrap()).map_err(|e| e.to_string())?;
        // rename replaces a file on Windows as well as Unix; never fall back to AppData.
        fs::rename(temp, destination).map_err(|e| e.to_string())
    }
}
pub fn portable_root() -> Result<PathBuf, String> {
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let directory = executable
        .parent()
        .ok_or("Executable has no parent directory")?;
    #[cfg(target_os = "macos")]
    let directory = if directory.file_name().is_some_and(|n| n == "MacOS") {
        directory
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .unwrap_or(directory)
    } else {
        directory
    };
    Ok(directory.join("runtime-data"))
}
pub fn prepare(root: &Path) -> Result<(), String> {
    for child in ["temp", "cache"] {
        fs::create_dir_all(root.join(child))
            .map_err(|e| format!("Cannot create portable data in {}: {e}", root.display()))?;
    }
    // Set only this process and its children. No registry or global environment edits.
    for name in ["TEMP", "TMP", "TMPDIR"] {
        std::env::set_var(name, root.join("temp"));
    }
    std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
    std::env::set_var("MESA_SHADER_CACHE_DIR", root.join("cache"));
    Ok(())
}
pub fn example(root: &Path, name: &str) -> Result<PathBuf, String> {
    let destination = root.join("examples").join(env!("CARGO_PKG_VERSION"));
    for (relative, bytes) in EXAMPLES {
        let path = destination.join(relative);
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        if !path.exists() {
            fs::write(path, bytes).map_err(|e| e.to_string())?;
        }
    }
    Ok(destination.join(name))
}
pub fn fonts() -> Result<FontDefinitions, String> {
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(include_bytes!(env!("REPOTOWER_FONT")).as_slice())
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("noto-cjk".into(), FontData::from_owned(bytes).into());
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .get_mut(&family)
            .unwrap()
            .push("noto-cjk".into());
    }
    Ok(fonts)
}
