#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use repotower_launcher::{cache_valid, extract, Payload, Result};
use std::fs::OpenOptions;
use std::{env, fs, process::Command};

fn run() -> Result<i32> {
    let executable = env::current_exe()?;
    let parent = executable
        .parent()
        .ok_or("Cannot locate the launcher directory")?;
    let data = parent.join("RepoTower-data");
    fs::create_dir_all(&data).map_err(|_| {
        "Move RepoTower to a writable folder. It stores its app cache and settings beside the EXE."
    })?;
    if fs::symlink_metadata(&data)?.file_type().is_symlink() {
        return Err("The portable data folder must not be a link".into());
    }
    // An OS file lock is released automatically even if extraction is interrupted.
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(data.join("extract.lock"))?;
    lock.lock()?;
    let mut payload = Payload::open(&executable)?;
    let key = payload.key();
    let target = data.join(format!("app-{}", &key[..16]));
    if target.exists() && fs::symlink_metadata(&target)?.file_type().is_symlink() {
        return Err("The application cache must not be a link".into());
    }
    payload.verify()?;
    let mut archive = zip::ZipArchive::new(payload)?;
    if !cache_valid(&mut archive, &target) {
        let stage = data.join(format!("extract-{}", std::process::id()));
        if stage.exists() {
            return Err("An incomplete extraction folder exists. Close RepoTower and remove RepoTower-data/extract-* before retrying.".into());
        }
        fs::create_dir(&stage)?;
        if let Err(error) = extract(&mut archive, &stage) {
            let _ = fs::remove_dir_all(&stage);
            return Err(error);
        }
        if target.exists() {
            fs::remove_dir_all(&target)?;
        }
        fs::rename(&stage, &target)?;
    }
    lock.unlock()?;
    let app = target.join("RepoTower.exe");
    if !app.is_file() {
        return Err("RepoTower.exe is missing from the application payload".into());
    }
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.iter().any(|arg| arg == "--repotower-extract-only") {
        return Ok(0);
    }
    let temp = data.join("runtime-data/tmp");
    fs::create_dir_all(&temp)?;
    let status = Command::new(app)
        .args(args)
        .current_dir(&target)
        .env("REPOTOWER_PORTABLE_ROOT", &data)
        .env("TEMP", &temp)
        .env("TMP", &temp)
        .env("TMPDIR", &temp)
        .status()?;
    Ok(status.code().unwrap_or(1))
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            let message = format!("RepoTower could not start.\n\n{error}\n\nKeep the EXE in a writable folder. Download a fresh copy if its integrity check failed.");
            #[cfg(target_os = "windows")]
            unsafe {
                #[link(name = "user32")]
                extern "system" {
                    fn MessageBoxW(
                        window: *mut std::ffi::c_void,
                        text: *const u16,
                        title: *const u16,
                        flags: u32,
                    ) -> i32;
                }
                let text: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
                let title: Vec<u16> = "RepoTower".encode_utf16().chain(Some(0)).collect();
                MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x10);
            }
            #[cfg(not(target_os = "windows"))]
            eprintln!("{message}");
            std::process::exit(1);
        }
    }
}
