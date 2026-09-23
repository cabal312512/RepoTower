use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Component, Path, PathBuf};

pub const FOOTER_SIZE: u64 = 64;
pub const MAGIC: &[u8; 8] = b"RTOWER01";
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct Payload {
    file: File,
    start: u64,
    length: u64,
    position: u64,
    pub digest: [u8; 32],
}
impl Payload {
    pub fn open(executable: &Path) -> Result<Self> {
        let mut file = File::open(executable)?;
        let size = file.metadata()?.len();
        if size < FOOTER_SIZE {
            return Err("Missing application payload".into());
        }
        file.seek(SeekFrom::End(-(FOOTER_SIZE as i64)))?;
        let mut footer = [0u8; FOOTER_SIZE as usize];
        file.read_exact(&mut footer)?;
        if &footer[..8] != MAGIC {
            return Err("Invalid application payload".into());
        }
        let start = u64::from_le_bytes(footer[8..16].try_into()?);
        let length = u64::from_le_bytes(footer[16..24].try_into()?);
        if start
            .checked_add(length)
            .and_then(|n| n.checked_add(FOOTER_SIZE))
            != Some(size)
            || length == 0
        {
            return Err("Incomplete application download".into());
        }
        let mut payload = Self {
            file,
            start,
            length,
            position: 0,
            digest: footer[24..56].try_into()?,
        };
        payload.seek(SeekFrom::Start(0))?;
        Ok(payload)
    }
    pub fn key(&self) -> String {
        self.digest
            .iter()
            .map(|value| format!("{value:02x}"))
            .collect()
    }
    pub fn verify(&mut self) -> Result<()> {
        self.seek(SeekFrom::Start(0))?;
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let count = self.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            digest.update(&buffer[..count]);
        }
        if digest.finalize().as_slice() != self.digest {
            return Err("The application download failed its integrity check".into());
        }
        self.seek(SeekFrom::Start(0))?;
        Ok(())
    }
}
impl Read for Payload {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let limit = buffer.len().min((self.length - self.position) as usize);
        let count = self.file.read(&mut buffer[..limit])?;
        self.position += count as u64;
        Ok(count)
    }
}
impl Seek for Payload {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let position = match from {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::Current(value) => self.position as i128 + value as i128,
            SeekFrom::End(value) => self.length as i128 + value as i128,
        };
        if position < 0 || position > self.length as i128 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek outside application payload",
            ));
        }
        self.file
            .seek(SeekFrom::Start(self.start + position as u64))?;
        self.position = position as u64;
        Ok(self.position)
    }
}

pub fn safe_path(name: &str) -> Result<PathBuf> {
    if name.starts_with(['/', '\\'])
        || name.contains(':')
        || name.contains('\0')
        || name.split(['/', '\\']).any(|part| part == "..")
    {
        return Err("Unsafe path in application archive".into());
    }
    let path = Path::new(name);
    if !path
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir))
        || path.as_os_str().is_empty()
    {
        return Err("Invalid path in application archive".into());
    }
    Ok(path.to_owned())
}

pub fn extract<R: Read + Seek>(archive: &mut zip::ZipArchive<R>, target: &Path) -> Result<()> {
    if archive.len() > 5000 {
        return Err("Application archive has too many entries".into());
    }
    let mut total = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let relative = safe_path(entry.name())?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("Links are not supported in the Windows payload".into());
        }
        total = total
            .checked_add(entry.size())
            .ok_or("Archive size overflow")?;
        if total > 2 * 1024 * 1024 * 1024 {
            return Err("Application archive exceeds size limit".into());
        }
        let destination = target.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(destination)?;
            continue;
        }
        fs::create_dir_all(destination.parent().ok_or("Missing parent directory")?)?;
        let mut output = File::create(destination)?;
        let written = io::copy(&mut entry, &mut output)?;
        if written != entry.size() {
            return Err("Incomplete application file".into());
        }
        output.flush()?;
    }
    Ok(())
}

pub fn cache_valid<R: Read + Seek>(archive: &mut zip::ZipArchive<R>, target: &Path) -> bool {
    (|| -> Result<bool> {
        for index in 0..archive.len() {
            let entry = archive.by_index(index)?;
            let relative = safe_path(entry.name())?;
            if entry.is_dir() {
                continue;
            }
            let file_path = target.join(relative);
            // Refuse redirected cache entries before reading or launching them.
            let mut parent = file_path.as_path();
            while parent != target {
                if fs::symlink_metadata(parent)?.file_type().is_symlink() {
                    return Ok(false);
                }
                parent = parent.parent().ok_or("Invalid cache path")?;
            }
            let mut file = File::open(file_path)?;
            if file.metadata()?.len() != entry.size() {
                return Ok(false);
            }
            let mut crc = crc32fast::Hasher::new();
            let mut buffer = [0u8; 65536];
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                crc.update(&buffer[..count]);
            }
            if crc.finalize() != entry.crc32() {
                return Ok(false);
            }
        }
        Ok(true)
    })()
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    #[test]
    fn rejects_absolute_parent_and_windows_escape_paths() {
        for path in [
            "../x",
            "a/../../x",
            "a\\..\\x",
            "/x",
            "\\server\\x",
            "C:/x",
            "a:x",
            "",
        ] {
            assert!(safe_path(path).is_err(), "{path}");
        }
        assert_eq!(
            safe_path("resources/app.asar").unwrap(),
            PathBuf::from("resources/app.asar")
        );
    }
    #[test]
    fn extracts_and_detects_missing_or_corrupted_cache_files() {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(
            "resources/app.asar",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"verified app").unwrap();
        let data = zip.finish().unwrap().into_inner();
        let mut archive = zip::ZipArchive::new(Cursor::new(data)).unwrap();
        let temp = tempfile::tempdir().unwrap();
        assert!(!cache_valid(&mut archive, temp.path()));
        extract(&mut archive, temp.path()).unwrap();
        assert!(cache_valid(&mut archive, temp.path()));
        fs::write(temp.path().join("resources/app.asar"), b"tampered app").unwrap();
        assert!(!cache_valid(&mut archive, temp.path()));
    }
    #[test]
    fn bounds_and_verifies_the_embedded_payload() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("launcher.exe");
        let mut data = b"stubpayload".to_vec();
        let mut footer = [0u8; 64];
        footer[..8].copy_from_slice(MAGIC);
        footer[8..16].copy_from_slice(&4u64.to_le_bytes());
        footer[16..24].copy_from_slice(&7u64.to_le_bytes());
        footer[24..56].copy_from_slice(&Sha256::digest(b"payload"));
        data.extend_from_slice(&footer);
        fs::write(&path, &data).unwrap();
        let mut payload = Payload::open(&path).unwrap();
        payload.verify().unwrap();
        assert!(payload.seek(SeekFrom::Start(8)).is_err());
        data[5] ^= 1;
        fs::write(&path, &data).unwrap();
        assert!(Payload::open(&path).unwrap().verify().is_err());
        data.pop();
        fs::write(&path, &data).unwrap();
        assert!(Payload::open(&path).is_err());
    }
}
