#![cfg(feature = "sftp")]

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use ssh2::{Session, Sftp};

use crate::backend::{ConnectionParams, ConnectionProtocol, DirEntry, FileInfo, FilesystemBackend};

fn sftp_err(op: &str, e: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::Other, format!("SFTP {}: {}", op, e))
}

pub struct SftpBackend {
    session: Mutex<Session>,
    sftp: Mutex<Sftp>,
    display: String,
    params: ConnectionParams,
}

// Safety: Session/Sftp are not Sync but we protect them with Mutex
unsafe impl Sync for SftpBackend {}

impl SftpBackend {
    pub fn connect(
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        key_path: Option<&str>,
    ) -> io::Result<Self> {
        let tcp = TcpStream::connect((host, port))?;
        let mut session = Session::new()
            .map_err(|e| sftp_err("create session", e))?;
        session.set_tcp_stream(tcp);
        session.handshake()
            .map_err(|e| io::Error::new(io::ErrorKind::ConnectionRefused, e.to_string()))?;
        session.set_timeout(30_000);

        // Try auth methods in order: key, password, ssh-agent
        // 1. Key-based auth (if key provided)
        if let Some(key) = key_path {
            let passphrase = if password.is_empty() { None } else { Some(password) };
            let _ = session.userauth_pubkey_file(username, None, Path::new(key), passphrase);
        }

        // 2. Password auth (if not yet authenticated and password provided)
        if !session.authenticated() && !password.is_empty() {
            let _ = session.userauth_password(username, password);
        }

        // 3. SSH agent (if nothing else worked)
        if !session.authenticated() {
            let _ = session.userauth_agent(username);
        }

        if !session.authenticated() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Authentication failed — tried key, password, and ssh-agent",
            ));
        }

        let sftp = session.sftp()
            .map_err(|e| sftp_err("open SFTP channel", e))?;

        Ok(Self {
            display: format!("SFTP: {}@{}:{}", username, host, port),
            session: Mutex::new(session),
            sftp: Mutex::new(sftp),
            params: ConnectionParams {
                protocol: ConnectionProtocol::Sftp,
                host: host.to_string(),
                port,
                username: username.to_string(),
                password: password.to_string(),
                key_path: key_path.map(|s| s.to_string()),
            },
        })
    }
}

fn filestat_to_fileinfo(name: &str, stat: &ssh2::FileStat) -> FileInfo {
    let is_dir = stat.is_dir();
    let is_file = stat.is_file();
    let size = stat.size.unwrap_or(0);
    let modified = stat.mtime.map(|t| UNIX_EPOCH + std::time::Duration::from_secs(t));
    let readonly = stat.perm.map(|p| p & 0o200 == 0).unwrap_or(false);

    let is_symlink = stat.perm.map(|p| (p & 0o170000) == 0o120000).unwrap_or(false);

    FileInfo {
        name: name.to_string(),
        is_dir,
        is_file,
        is_symlink,
        size,
        modified,
        readonly,
        mode: stat.perm,
    }
}

impl FilesystemBackend for SftpBackend {
    fn display_name(&self) -> String {
        self.display.clone()
    }

    fn is_local(&self) -> bool {
        false
    }

    fn list_dir(&self, path: &str) -> io::Result<Vec<DirEntry>> {
        let sftp = self.sftp.lock().map_err(|e| sftp_err("lock", e))?;
        let entries = sftp.readdir(Path::new(path))
            .map_err(|e| sftp_err("list directory", e))?;

        let mut result = Vec::new();
        for (pathbuf, stat) in entries {
            let name = pathbuf.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name == "." || name == ".." {
                continue;
            }
            result.push(DirEntry {
                name: name.clone(),
                info: filestat_to_fileinfo(&name, &stat),
            });
        }
        Ok(result)
    }

    fn metadata(&self, path: &str) -> io::Result<FileInfo> {
        let sftp = self.sftp.lock().map_err(|e| sftp_err("lock", e))?;
        let stat = sftp.stat(Path::new(path))
            .map_err(|e| sftp_err("read metadata", e))?;
        let name = Path::new(path).file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        Ok(filestat_to_fileinfo(&name, &stat))
    }

    fn exists(&self, path: &str) -> io::Result<bool> {
        let sftp = self.sftp.lock().map_err(|e| sftp_err("lock", e))?;
        Ok(sftp.stat(Path::new(path)).is_ok())
    }

    fn canonicalize(&self, path: &str) -> io::Result<String> {
        let sftp = self.sftp.lock().map_err(|e| sftp_err("lock", e))?;
        let real = sftp.realpath(Path::new(path))
            .map_err(|e| sftp_err("resolve path", e))?;
        Ok(real.to_string_lossy().to_string())
    }

    fn read_file(&self, path: &str, max_bytes: usize) -> io::Result<Vec<u8>> {
        let sftp = self.sftp.lock().map_err(|e| sftp_err("lock", e))?;
        let mut file = sftp.open(Path::new(path))
            .map_err(|e| sftp_err("open file", e))?;
        let mut buffer = vec![0u8; max_bytes.min(1024 * 1024)]; // cap at 1MB
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);
        Ok(buffer)
    }

    fn write_file(&self, path: &str, data: &[u8]) -> io::Result<()> {
        let sftp = self.sftp.lock().map_err(|e| sftp_err("lock", e))?;
        let mut file = sftp.create(Path::new(path))
            .map_err(|e| sftp_err("create file", e))?;
        file.write_all(data)?;
        Ok(())
    }

    fn create_dir(&self, path: &str) -> io::Result<()> {
        let sftp = self.sftp.lock().map_err(|e| sftp_err("lock", e))?;
        sftp.mkdir(Path::new(path), 0o755)
            .map_err(|e| sftp_err("create directory", e))
    }

    fn create_file(&self, path: &str) -> io::Result<()> {
        self.write_file(path, &[])
    }

    fn remove_file(&self, path: &str) -> io::Result<()> {
        let sftp = self.sftp.lock().map_err(|e| sftp_err("lock", e))?;
        sftp.unlink(Path::new(path))
            .map_err(|e| sftp_err("delete file", e))
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
        let sftp = self.sftp.lock().map_err(|e| sftp_err("lock", e))?;
        sftp.rmdir(Path::new(path))
            .map_err(|e| sftp_err("remove directory", e))
    }

    fn rename(&self, from: &str, to: &str) -> io::Result<()> {
        let sftp = self.sftp.lock().map_err(|e| sftp_err("lock", e))?;
        sftp.rename(Path::new(from), Path::new(to), None)
            .map_err(|e| sftp_err("rename", e))
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

    fn disconnect(&self) {
        if let Ok(session) = self.session.lock() {
            let _ = session.disconnect(None, "Client disconnected", None);
        }
    }

    fn chmod(&self, path: &str, mode: u32) -> io::Result<()> {
        let sftp = self.sftp.lock().map_err(|e| sftp_err("lock", e))?;
        let stat = ssh2::FileStat {
            size: None,
            uid: None,
            gid: None,
            perm: Some(mode),
            atime: None,
            mtime: None,
        };
        sftp.setstat(Path::new(path), stat)
            .map_err(|e| sftp_err("chmod", e))
    }

    fn is_connected(&self) -> bool {
        if let Ok(session) = self.session.lock() {
            session.authenticated()
        } else {
            false
        }
    }

    fn connection_params(&self) -> Option<ConnectionParams> {
        Some(self.params.clone())
    }
}
