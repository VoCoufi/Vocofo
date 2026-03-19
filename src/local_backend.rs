use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use crate::backend::{DirEntry, FileInfo, FilesystemBackend};

pub struct LocalBackend;

impl LocalBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalBackend {
    fn default() -> Self {
        Self::new()
    }
}

fn metadata_to_fileinfo(name: &str, meta: &fs::Metadata) -> FileInfo {
    #[cfg(unix)]
    let mode = Some(meta.permissions().mode());
    #[cfg(not(unix))]
    let mode = None;
    FileInfo {
        name: name.to_string(),
        is_dir: meta.is_dir(),
        is_file: meta.is_file(),
        is_symlink: meta.is_symlink(),
        size: meta.len(),
        modified: meta.modified().ok(),
        readonly: meta.permissions().readonly(),
        mode,
    }
}

impl FilesystemBackend for LocalBackend {
    fn display_name(&self) -> String {
        "Local".to_string()
    }

    fn is_local(&self) -> bool {
        true
    }

    fn list_dir(&self, path: &str) -> io::Result<Vec<DirEntry>> {
        let mut entries = Vec::new();

        for entry_result in fs::read_dir(path)? {
            let entry = entry_result?;
            let file_name = entry.file_name()
                .into_string()
                .map_err(|_| io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid UTF-8 in filename",
                ))?;
            let meta = entry.metadata()?;
            entries.push(DirEntry {
                name: file_name.clone(),
                info: metadata_to_fileinfo(&file_name, &meta),
            });
        }

        Ok(entries)
    }

    fn metadata(&self, path: &str) -> io::Result<FileInfo> {
        let p = Path::new(path);
        let meta = fs::metadata(p)?;
        let name = p.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok(metadata_to_fileinfo(&name, &meta))
    }

    fn exists(&self, path: &str) -> io::Result<bool> {
        Ok(Path::new(path).exists())
    }

    fn canonicalize(&self, path: &str) -> io::Result<String> {
        let canonical = fs::canonicalize(path)?;
        canonical.to_str()
            .map(|s| s.to_string())
            .ok_or_else(|| io::Error::new(
                io::ErrorKind::InvalidData,
                "Path contains invalid Unicode",
            ))
    }

    fn read_file(&self, path: &str, max_bytes: usize) -> io::Result<Vec<u8>> {
        let mut file = fs::File::open(path)?;
        let mut buffer = vec![0u8; max_bytes];
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    fn write_file(&self, path: &str, data: &[u8]) -> io::Result<()> {
        fs::write(path, data)
    }

    fn create_dir(&self, path: &str) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn create_file(&self, path: &str) -> io::Result<()> {
        fs::File::create(path)?;
        Ok(())
    }

    fn remove_file(&self, path: &str) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn remove_dir_all(&self, path: &str) -> io::Result<()> {
        fs::remove_dir_all(path)
    }

    fn rename(&self, from: &str, to: &str) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn copy_file(&self, from: &str, to: &str) -> io::Result<()> {
        if let Some(parent) = Path::new(to).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from, to)?;
        Ok(())
    }

    fn copy_dir(&self, from: &str, to: &str) -> io::Result<()> {
        let src = Path::new(from);
        let dst = Path::new(to);

        if !src.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Source is not a directory",
            ));
        }

        if !dst.exists() {
            fs::create_dir_all(dst)?;
        }

        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            let from_path = entry.path();
            let to_path = dst.join(entry.file_name());

            if file_type.is_dir() {
                self.copy_dir(
                    &from_path.to_string_lossy(),
                    &to_path.to_string_lossy(),
                )?;
            } else if file_type.is_file() {
                self.copy_file(
                    &from_path.to_string_lossy(),
                    &to_path.to_string_lossy(),
                )?;
            } else if file_type.is_symlink() {
                let target = fs::read_link(&from_path)?;
                let resolved = if target.is_absolute() {
                    target
                } else {
                    from_path.parent().unwrap_or(Path::new("")).join(target)
                };
                if resolved.is_dir() {
                    self.copy_dir(
                        &resolved.to_string_lossy(),
                        &to_path.to_string_lossy(),
                    )?;
                } else {
                    self.copy_file(
                        &resolved.to_string_lossy(),
                        &to_path.to_string_lossy(),
                    )?;
                }
            }
        }

        Ok(())
    }

    fn join_path(&self, base: &str, child: &str) -> String {
        PathBuf::from(base).join(child).to_string_lossy().to_string()
    }

    fn parent_path(&self, path: &str) -> Option<String> {
        Path::new(path).parent().map(|p| p.to_string_lossy().to_string())
    }

    fn file_name(&self, path: &str) -> Option<String> {
        Path::new(path).file_name().map(|n| n.to_string_lossy().to_string())
    }

    #[cfg(unix)]
    fn chmod(&self, path: &str, mode: u32) -> io::Result<()> {
        let perms = fs::Permissions::from_mode(mode);
        fs::set_permissions(path, perms)
    }
}
