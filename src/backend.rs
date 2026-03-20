use std::io;
use std::time::SystemTime;

/// Metadata for a single file/directory entry — filesystem-agnostic
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub readonly: bool,
    /// Raw unix permission mode (e.g. 0o755), None if unavailable
    pub mode: Option<u32>,
}

/// A single directory entry with its metadata
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub info: FileInfo,
}

/// Stored connection parameters for reconnection
#[derive(Debug, Clone)]
pub struct ConnectionParams {
    pub protocol: ConnectionProtocol,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub key_path: Option<String>,
}

/// Connection protocol for remote backends
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionProtocol {
    Sftp,
    Ftp,
}

/// Abstraction over filesystem operations — local, SFTP, FTP, etc.
pub trait FilesystemBackend: Send + Sync {
    /// Display name for UI ("Local", "SFTP: user@host", etc.)
    fn display_name(&self) -> String;

    /// Whether this is a local filesystem (affects file opening behavior)
    fn is_local(&self) -> bool;

    /// List directory contents
    fn list_dir(&self, path: &str) -> io::Result<Vec<DirEntry>>;

    /// Get metadata for a single path
    fn metadata(&self, path: &str) -> io::Result<FileInfo>;

    /// Check if a path exists
    fn exists(&self, path: &str) -> io::Result<bool>;

    /// Canonicalize/resolve a path (resolve "..", ".", symlinks)
    fn canonicalize(&self, path: &str) -> io::Result<String>;

    /// Read file contents up to max_bytes
    fn read_file(&self, path: &str, max_bytes: usize) -> io::Result<Vec<u8>>;

    /// Write data to a file (create or overwrite)
    fn write_file(&self, path: &str, data: &[u8]) -> io::Result<()>;

    /// Create a directory (including parents)
    fn create_dir(&self, path: &str) -> io::Result<()>;

    /// Create an empty file
    fn create_file(&self, path: &str) -> io::Result<()>;

    /// Remove a single file
    fn remove_file(&self, path: &str) -> io::Result<()>;

    /// Remove a directory recursively
    fn remove_dir_all(&self, path: &str) -> io::Result<()>;

    /// Rename/move within the same backend
    fn rename(&self, from: &str, to: &str) -> io::Result<()>;

    /// Copy a single file within the same backend (default: read + write)
    fn copy_file(&self, from: &str, to: &str) -> io::Result<()> {
        let data = self.read_file(from, usize::MAX)?;
        self.write_file(to, &data)
    }

    /// Copy a directory recursively within the same backend (default: recursive read + write)
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

    /// Join a base path and a child into a full path
    fn join_path(&self, base: &str, child: &str) -> String {
        if base.ends_with('/') {
            format!("{}{}", base, child)
        } else {
            format!("{}/{}", base, child)
        }
    }

    /// Get the parent directory of a path
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

    /// Get the file/directory name from a path
    fn file_name(&self, path: &str) -> Option<String> {
        let path = path.trim_end_matches('/');
        path.rsplit('/').next().map(|s| s.to_string())
    }

    /// Explicitly close the connection (no-op for local backend)
    fn disconnect(&self) {}

    /// Change file permissions (octal mode, e.g. 0o755)
    fn chmod(&self, _path: &str, _mode: u32) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "chmod not supported",
        ))
    }

    /// Check if the connection is still alive (always true for local)
    fn is_connected(&self) -> bool {
        true
    }

    /// Get connection parameters for reconnection (None for local)
    fn connection_params(&self) -> Option<ConnectionParams> {
        None
    }
}
