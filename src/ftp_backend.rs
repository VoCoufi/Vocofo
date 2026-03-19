#![cfg(feature = "ftp")]

use std::io::{self, Cursor};
use std::sync::Mutex;

use suppaftp::FtpStream;

use crate::backend::{DirEntry, FileInfo, FilesystemBackend};

pub struct FtpBackend {
    ftp: Mutex<FtpStream>,
    display: String,
}

// Safety: FtpStream is not Sync but we protect it with Mutex
unsafe impl Sync for FtpBackend {}

impl FtpBackend {
    pub fn connect(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> io::Result<Self> {
        let mut ftp = FtpStream::connect(format!("{}:{}", host, port))
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;
        ftp.login(username, password)
            .map_err(|e| io::Error::new(io::ErrorKind::PermissionDenied, e.to_string()))?;

        Ok(Self {
            display: format!("FTP: {}@{}:{}", username, host, port),
            ftp: Mutex::new(ftp),
        })
    }
}

/// Parse a unix-style FTP LIST line into a DirEntry
fn parse_list_line(line: &str) -> Option<DirEntry> {
    // Format: drwxr-xr-x 2 user group 4096 Jan 01 12:00 filename
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 9 {
        return None;
    }

    let perms = parts[0];
    let is_dir = perms.starts_with('d');
    let is_symlink = perms.starts_with('l');
    let is_file = !is_dir && !is_symlink;
    let size: u64 = parts[4].parse().unwrap_or(0);
    let name = parts[8..].join(" ");

    // Skip . and ..
    if name == "." || name == ".." {
        return None;
    }

    // For symlinks, strip " -> target"
    let clean_name = if is_symlink {
        name.split(" -> ").next().unwrap_or(&name).to_string()
    } else {
        name
    };

    Some(DirEntry {
        name: clean_name.clone(),
        info: FileInfo {
            name: clean_name,
            is_dir,
            is_file,
            is_symlink,
            size,
            modified: None,
            readonly: false,
        },
    })
}

impl FilesystemBackend for FtpBackend {
    fn display_name(&self) -> String {
        self.display.clone()
    }

    fn is_local(&self) -> bool {
        false
    }

    fn list_dir(&self, path: &str) -> io::Result<Vec<DirEntry>> {
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let lines = ftp.list(Some(path))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let entries: Vec<DirEntry> = lines.iter()
            .filter_map(|line| parse_list_line(line))
            .collect();

        Ok(entries)
    }

    fn metadata(&self, path: &str) -> io::Result<FileInfo> {
        // Try to get size (works for files)
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let size = ftp.size(path).unwrap_or(0);

        // Try to determine if directory by listing parent
        let name = path.rsplit('/').next().unwrap_or(path).to_string();

        // Try cwd to check if it's a directory
        let is_dir = if let Ok(pwd) = ftp.pwd() {
            if ftp.cwd(path).is_ok() {
                let _ = ftp.cwd(&pwd); // restore
                true
            } else {
                false
            }
        } else {
            false
        };

        Ok(FileInfo {
            name,
            is_dir,
            is_file: !is_dir,
            is_symlink: false,
            size: size as u64,
            modified: None,
            readonly: false,
        })
    }

    fn exists(&self, path: &str) -> io::Result<bool> {
        // Try size (file) or cwd (dir)
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        if ftp.size(path).is_ok() {
            return Ok(true);
        }
        if let Ok(pwd) = ftp.pwd() {
            if ftp.cwd(path).is_ok() {
                let _ = ftp.cwd(&pwd);
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn canonicalize(&self, path: &str) -> io::Result<String> {
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        // Navigate to path and get absolute path
        let original = ftp.pwd()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        ftp.cwd(path)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let real = ftp.pwd()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let _ = ftp.cwd(&original);
        Ok(real)
    }

    fn read_file(&self, path: &str, max_bytes: usize) -> io::Result<Vec<u8>> {
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let cursor = ftp.retr_as_buffer(path)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let data = cursor.into_inner();
        if data.len() > max_bytes {
            Ok(data[..max_bytes].to_vec())
        } else {
            Ok(data)
        }
    }

    fn write_file(&self, path: &str, data: &[u8]) -> io::Result<()> {
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let mut cursor = Cursor::new(data);
        ftp.put_file(path, &mut cursor)
            .map(|_| ())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    fn create_dir(&self, path: &str) -> io::Result<()> {
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        ftp.mkdir(path)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(())
    }

    fn create_file(&self, path: &str) -> io::Result<()> {
        self.write_file(path, &[])
    }

    fn remove_file(&self, path: &str) -> io::Result<()> {
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        ftp.rm(path)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    fn remove_dir_all(&self, path: &str) -> io::Result<()> {
        let entries = self.list_dir(path)?;
        for entry in entries {
            let child = self.join_path(path, &entry.name);
            if entry.info.is_dir {
                self.remove_dir_all(&child)?;
            } else {
                self.remove_file(&child)?;
            }
        }
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        ftp.rmdir(path)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    fn rename(&self, from: &str, to: &str) -> io::Result<()> {
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        ftp.rename(from, to)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    fn copy_file(&self, from: &str, to: &str) -> io::Result<()> {
        let data = self.read_file(from, usize::MAX)?;
        self.write_file(to, &data)
    }

    fn copy_dir(&self, from: &str, to: &str) -> io::Result<()> {
        self.create_dir(to)?;
        let entries = self.list_dir(from)?;
        for entry in entries {
            let src = self.join_path(from, &entry.name);
            let dst = self.join_path(to, &entry.name);
            if entry.info.is_dir {
                self.copy_dir(&src, &dst)?;
            } else {
                self.copy_file(&src, &dst)?;
            }
        }
        Ok(())
    }

    fn join_path(&self, base: &str, child: &str) -> String {
        if base.ends_with('/') {
            format!("{}{}", base, child)
        } else {
            format!("{}/{}", base, child)
        }
    }

    fn parent_path(&self, path: &str) -> Option<String> {
        let path = path.trim_end_matches('/');
        if path.is_empty() || path == "/" {
            return None;
        }
        match path.rfind('/') {
            Some(0) => Some("/".to_string()),
            Some(pos) => Some(path[..pos].to_string()),
            None => Some("/".to_string()),
        }
    }

    fn file_name(&self, path: &str) -> Option<String> {
        let path = path.trim_end_matches('/');
        path.rsplit('/').next().map(|s| s.to_string())
    }
}
