#![cfg(feature = "sftp")]

use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::Mutex;

use ssh2::Session;

use crate::backend::{ConnectionParams, DirEntry, FileInfo, FilesystemBackend};

fn scp_err(op: &str, e: impl std::fmt::Display) -> io::Error {
    io::Error::other(format!("SCP {}: {}", op, e))
}
#[cfg(feature = "ftp")]
use crate::ftp_backend::parse_list_line;

/// SCP fallback backend — used when SFTP subsystem is unavailable.
/// Uses SSH exec for browsing and SCP for file transfers.
pub struct ScpBackend {
    session: Mutex<Session>,
    display: String,
    params: ConnectionParams,
}

// Safety: Session is not Sync but we protect it with Mutex
unsafe impl Sync for ScpBackend {}

impl ScpBackend {
    pub fn from_session(session: Session, params: ConnectionParams) -> Self {
        let display = format!("SCP: {}@{}:{}", params.username, params.host, params.port);
        Self {
            session: Mutex::new(session),
            display,
            params,
        }
    }

    /// Execute a command via SSH and return stdout
    fn ssh_exec(&self, cmd: &str) -> io::Result<String> {
        let session = self.session.lock().map_err(|e| scp_err("operation", e))?;
        let mut channel = session
            .channel_session()
            .map_err(|e| scp_err("operation", e))?;
        channel.exec(cmd).map_err(|e| scp_err("operation", e))?;
        let mut output = String::new();
        channel
            .read_to_string(&mut output)
            .map_err(|e| scp_err("operation", e))?;
        channel.wait_close().map_err(|e| scp_err("operation", e))?;
        Ok(output)
    }
}

/// Shell-escape a path for use in SSH commands
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Parse `ls -la` output line into a DirEntry (reuses FTP LIST parsing logic)
fn parse_ls_line(line: &str) -> Option<DirEntry> {
    // ls -la format is same as FTP LIST: "drwxr-xr-x 2 user group 4096 Jan 01 12:00 filename"
    // We can reuse the FTP parser if available, otherwise parse manually
    #[cfg(feature = "ftp")]
    {
        parse_list_line(line)
    }
    #[cfg(not(feature = "ftp"))]
    {
        parse_ls_line_internal(line)
    }
}

#[cfg(not(feature = "ftp"))]
fn parse_ls_line_internal(line: &str) -> Option<DirEntry> {
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
    if name == "." || name == ".." {
        return None;
    }
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
            mode: None,
        },
    })
}

impl FilesystemBackend for ScpBackend {
    fn display_name(&self) -> String {
        self.display.clone()
    }

    fn is_local(&self) -> bool {
        false
    }

    fn list_dir(&self, path: &str) -> io::Result<Vec<DirEntry>> {
        let output = self.ssh_exec(&format!("ls -la {}", shell_escape(path)))?;
        let entries: Vec<DirEntry> = output.lines().filter_map(parse_ls_line).collect();
        Ok(entries)
    }

    fn metadata(&self, path: &str) -> io::Result<FileInfo> {
        // Use stat to get metadata
        let output = self.ssh_exec(&format!(
            "stat --format='%s %F %a' {} 2>/dev/null || echo 'NOTFOUND'",
            shell_escape(path)
        ))?;
        let trimmed = output.trim();
        if trimmed == "NOTFOUND" || trimmed.is_empty() {
            return Err(io::Error::new(io::ErrorKind::NotFound, "File not found"));
        }

        let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
        let size: u64 = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let file_type = parts.get(1).unwrap_or(&"");
        let mode_str = parts.get(2).unwrap_or(&"");

        let is_dir = file_type.contains("directory");
        let is_symlink = file_type.contains("symbolic");
        let is_file = !is_dir && !is_symlink;
        let mode = u32::from_str_radix(mode_str, 8).ok();
        let readonly = mode.map(|m| m & 0o200 == 0).unwrap_or(false);

        let name = Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(FileInfo {
            name,
            is_dir,
            is_file,
            is_symlink,
            size,
            modified: None,
            readonly,
            mode,
        })
    }

    fn exists(&self, path: &str) -> io::Result<bool> {
        let output = self.ssh_exec(&format!(
            "test -e {} && echo 1 || echo 0",
            shell_escape(path)
        ))?;
        Ok(output.trim() == "1")
    }

    fn canonicalize(&self, path: &str) -> io::Result<String> {
        let output = self.ssh_exec(&format!(
            "readlink -f {} 2>/dev/null || echo {}",
            shell_escape(path),
            shell_escape(path)
        ))?;
        let result = output.trim().to_string();
        if result.is_empty() {
            Ok(path.to_string())
        } else {
            Ok(result)
        }
    }

    fn read_file(&self, path: &str, max_bytes: usize) -> io::Result<Vec<u8>> {
        let session = self.session.lock().map_err(|e| scp_err("operation", e))?;
        let (mut channel, _stat) = session
            .scp_recv(Path::new(path))
            .map_err(|e| scp_err("operation", e))?;
        let mut data = Vec::new();
        channel
            .read_to_end(&mut data)
            .map_err(|e| scp_err("operation", e))?;
        if data.len() > max_bytes {
            data.truncate(max_bytes);
        }
        Ok(data)
    }

    fn write_file(&self, path: &str, data: &[u8]) -> io::Result<()> {
        let session = self.session.lock().map_err(|e| scp_err("operation", e))?;
        let mut channel = session
            .scp_send(Path::new(path), 0o644, data.len() as u64, None)
            .map_err(|e| scp_err("operation", e))?;
        channel
            .write_all(data)
            .map_err(|e| scp_err("operation", e))?;
        channel.send_eof().map_err(|e| scp_err("operation", e))?;
        channel.wait_eof().map_err(|e| scp_err("operation", e))?;
        channel.close().map_err(|e| scp_err("operation", e))?;
        channel.wait_close().map_err(|e| scp_err("operation", e))?;
        Ok(())
    }

    fn create_dir(&self, path: &str) -> io::Result<()> {
        self.ssh_exec(&format!("mkdir -p {}", shell_escape(path)))?;
        Ok(())
    }

    fn create_file(&self, path: &str) -> io::Result<()> {
        self.ssh_exec(&format!("touch {}", shell_escape(path)))?;
        Ok(())
    }

    fn remove_file(&self, path: &str) -> io::Result<()> {
        self.ssh_exec(&format!("rm {}", shell_escape(path)))?;
        Ok(())
    }

    fn remove_dir_all(&self, path: &str) -> io::Result<()> {
        self.ssh_exec(&format!("rm -rf {}", shell_escape(path)))?;
        Ok(())
    }

    fn rename(&self, from: &str, to: &str) -> io::Result<()> {
        self.ssh_exec(&format!("mv {} {}", shell_escape(from), shell_escape(to)))?;
        Ok(())
    }

    fn copy_file(&self, from: &str, to: &str) -> io::Result<()> {
        self.ssh_exec(&format!("cp {} {}", shell_escape(from), shell_escape(to)))?;
        Ok(())
    }

    fn copy_dir(&self, from: &str, to: &str) -> io::Result<()> {
        self.ssh_exec(&format!(
            "cp -r {} {}",
            shell_escape(from),
            shell_escape(to)
        ))?;
        Ok(())
    }

    // join_path, parent_path, file_name use trait defaults

    fn disconnect(&self) {
        if let Ok(session) = self.session.lock() {
            let _ = session.disconnect(None, "Client disconnected", None);
        }
    }

    fn chmod(&self, path: &str, mode: u32) -> io::Result<()> {
        self.ssh_exec(&format!("chmod {:o} {}", mode, shell_escape(path)))?;
        Ok(())
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
