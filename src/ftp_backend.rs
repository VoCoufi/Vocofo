#![cfg(feature = "ftp")]

use std::io::{self, Cursor};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use suppaftp::FtpStream;

use crate::backend::{ConnectionParams, ConnectionProtocol, DirEntry, FileInfo, FilesystemBackend};

/// Parse FTP LIST date fields ("Jan", "01", "12:00" or "2024") into SystemTime
fn parse_ftp_date(month: &str, day: &str, time_or_year: &str) -> Option<SystemTime> {
    let month_num = match month {
        "Jan" => 1, "Feb" => 2, "Mar" => 3, "Apr" => 4,
        "May" => 5, "Jun" => 6, "Jul" => 7, "Aug" => 8,
        "Sep" => 9, "Oct" => 10, "Nov" => 11, "Dec" => 12,
        _ => return None,
    };
    let day_num: u32 = day.parse().ok()?;

    let (year, hour, minute) = if time_or_year.contains(':') {
        // Format "HH:MM" — assume current year
        let mut parts = time_or_year.split(':');
        let h: u32 = parts.next()?.parse().ok()?;
        let m: u32 = parts.next()?.parse().ok()?;
        // Approximate current year from SystemTime
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
        let approx_year = 1970 + (now.as_secs() / 31_557_600); // ~365.25 days
        (approx_year as i32, h, m)
    } else {
        // Format "2024" — year, no time
        let y: i32 = time_or_year.parse().ok()?;
        (y, 0, 0)
    };

    // Convert to seconds since epoch (simplified, no leap seconds)
    let mut days: i64 = 0;
    for y in 1970..year {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    let month_days = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for m in 1..month_num {
        days += month_days[m as usize] as i64;
        if m == 2 && is_leap_year(year) {
            days += 1;
        }
    }
    days += (day_num - 1) as i64;
    let secs = days * 86400 + hour as i64 * 3600 + minute as i64 * 60;
    UNIX_EPOCH.checked_add(Duration::from_secs(secs as u64))
}

fn is_leap_year(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// Parse "rwxr-xr-x" style permission string to octal mode (e.g. 0o755)
fn parse_perm_string(perms: &str) -> Option<u32> {
    let chars: Vec<char> = perms.chars().collect();
    if chars.len() < 10 {
        return None;
    }
    let triplet = |r: char, w: char, x: char| -> u32 {
        (if r != '-' { 4 } else { 0 })
            + (if w != '-' { 2 } else { 0 })
            + (if x != '-' && x != 's' && x != 'S' && x != 't' && x != 'T' { 1 } else { 0 })
            + (if x == 's' || x == 't' { 1 } else { 0 })
    };
    let owner = triplet(chars[1], chars[2], chars[3]);
    let group = triplet(chars[4], chars[5], chars[6]);
    let other = triplet(chars[7], chars[8], chars[9]);
    Some(owner * 64 + group * 8 + other)
}

pub struct FtpBackend {
    ftp: Mutex<FtpStream>,
    display: String,
    params: ConnectionParams,
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
            params: ConnectionParams {
                protocol: ConnectionProtocol::Ftp,
                host: host.to_string(),
                port,
                username: username.to_string(),
                password: password.to_string(),
                key_path: None,
            },
        })
    }
}

/// Parse a unix-style FTP LIST line into a DirEntry
pub fn parse_list_line(line: &str) -> Option<DirEntry> {
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

    // Parse modification date from LIST output
    let modified = parse_ftp_date(parts[5], parts[6], parts[7]);

    // Parse readonly from owner write bit (position 2 in permission string)
    let readonly = perms.chars().nth(2) == Some('-');

    // Parse permission string to octal mode
    let mode = parse_perm_string(perms);

    Some(DirEntry {
        name: clean_name.clone(),
        info: FileInfo {
            name: clean_name,
            is_dir,
            is_file,
            is_symlink,
            size,
            modified,
            readonly,
            mode,
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
        // Get accurate metadata by listing the parent directory and finding our entry
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        let parent = self.parent_path(path).unwrap_or_else(|| "/".to_string());

        let entries = self.list_dir(&parent)?;
        if let Some(entry) = entries.into_iter().find(|e| e.name == name) {
            return Ok(entry.info);
        }

        // Fallback: might be a directory not found in listing (e.g., root)
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let is_dir = if let Ok(pwd) = ftp.pwd() {
            if ftp.cwd(path).is_ok() {
                let _ = ftp.cwd(&pwd);
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
            size: 0,
            modified: None,
            readonly: false,
            mode: None,
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

    fn disconnect(&self) {
        if let Ok(mut ftp) = self.ftp.lock() {
            let _ = ftp.quit();
        }
    }

    fn chmod(&self, path: &str, mode: u32) -> io::Result<()> {
        let mut ftp = self.ftp.lock()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        ftp.site(&format!("CHMOD {:o} {}", mode, path))
            .map(|_| ())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    fn is_connected(&self) -> bool {
        if let Ok(mut ftp) = self.ftp.lock() {
            ftp.noop().is_ok()
        } else {
            false
        }
    }

    fn connection_params(&self) -> Option<ConnectionParams> {
        Some(self.params.clone())
    }
}
