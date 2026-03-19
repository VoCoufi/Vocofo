#![cfg(feature = "sftp")]

//! Integration test for SftpBackend.
//! Starts a local sshd on a high port with temp keys, runs all SFTP operations, then cleans up.
//! Run with: cargo test --test sftp_integration_test -- --ignored --test-threads=1

use std::fs;
use std::io::Write;
use std::net::TcpListener;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

use tempfile::TempDir;
use vocofo::backend::FilesystemBackend;
use vocofo::sftp_backend::SftpBackend;

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

struct SftpTestHarness {
    pub backend: SftpBackend,
    pub data_dir: TempDir,
    sshd_process: Child,
    _config_dir: TempDir,
}

impl SftpTestHarness {
    fn setup() -> Result<Self, String> {
        let config_dir = TempDir::new().map_err(|e| format!("tempdir: {}", e))?;
        let data_dir = TempDir::new().map_err(|e| format!("tempdir: {}", e))?;
        let config_path = config_dir.path();

        let host_key = config_path.join("ssh_host_ed25519_key");
        let user_key = config_path.join("test_key");
        let authorized_keys = config_path.join("authorized_keys");
        let pid_file = config_path.join("sshd.pid");

        let status = Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-f"])
            .arg(&host_key)
            .args(["-N", "", "-q"])
            .status()
            .map_err(|e| format!("ssh-keygen host: {}", e))?;
        if !status.success() {
            return Err("Failed to generate host key".into());
        }

        let status = Command::new("ssh-keygen")
            .args(["-t", "ed25519", "-f"])
            .arg(&user_key)
            .args(["-N", "", "-q"])
            .status()
            .map_err(|e| format!("ssh-keygen user: {}", e))?;
        if !status.success() {
            return Err("Failed to generate user key".into());
        }

        let pubkey = fs::read_to_string(format!("{}.pub", user_key.display()))
            .map_err(|e| format!("read pubkey: {}", e))?;
        fs::write(&authorized_keys, &pubkey)
            .map_err(|e| format!("write authorized_keys: {}", e))?;

        let port = free_port();

        let sftp_server = if std::path::Path::new("/usr/lib/ssh/sftp-server").exists() {
            "/usr/lib/ssh/sftp-server"
        } else if std::path::Path::new("/usr/libexec/sftp-server").exists() {
            "/usr/libexec/sftp-server"
        } else {
            "internal-sftp"
        };

        let whoami = Command::new("whoami")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|_| "nobody".to_string());

        let sshd_config = config_path.join("sshd_config");
        let config_content = format!(
            r#"Port {port}
ListenAddress 127.0.0.1
HostKey {host_key}
PidFile {pid_file}
AuthorizedKeysFile {auth_keys}
StrictModes no
PasswordAuthentication no
PubkeyAuthentication yes
UsePAM no
Subsystem sftp {sftp_server}
AllowUsers {user}
LogLevel ERROR
"#,
            port = port,
            host_key = host_key.display(),
            pid_file = pid_file.display(),
            auth_keys = authorized_keys.display(),
            sftp_server = sftp_server,
            user = whoami,
        );

        let mut f =
            fs::File::create(&sshd_config).map_err(|e| format!("write sshd_config: {}", e))?;
        f.write_all(config_content.as_bytes())
            .map_err(|e| format!("write sshd_config: {}", e))?;

        let sshd_process = Command::new("/usr/bin/sshd")
            .arg("-D")
            .arg("-f")
            .arg(&sshd_config)
            .arg("-e")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("spawn sshd: {}", e))?;

        let mut ready = false;
        for _ in 0..50 {
            thread::sleep(Duration::from_millis(100));
            if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                ready = true;
                break;
            }
        }

        if !ready {
            // Try to get sshd error output
            let mut proc = sshd_process;
            let _ = proc.kill();
            let output = proc.wait_with_output().ok();
            let stderr = output
                .as_ref()
                .map(|o| String::from_utf8_lossy(&o.stderr).to_string())
                .unwrap_or_default();
            return Err(format!(
                "sshd did not start on port {}. stderr: {}",
                port, stderr
            ));
        }

        let backend = SftpBackend::connect(
            "127.0.0.1",
            port,
            &whoami,
            "",
            Some(&user_key.to_string_lossy()),
        )
        .map_err(|e| format!("SFTP connect: {}", e))?;

        Ok(Self {
            backend,
            data_dir,
            sshd_process,
            _config_dir: config_dir,
        })
    }

    fn data_path(&self) -> String {
        self.data_dir.path().to_string_lossy().to_string()
    }
}

impl Drop for SftpTestHarness {
    fn drop(&mut self) {
        let _ = self.sshd_process.kill();
        let _ = self.sshd_process.wait();
    }
}

/// Single test that exercises ALL SFTP operations sequentially with one sshd instance
#[test]
#[ignore]
fn test_sftp_all_operations() {
    let harness = SftpTestHarness::setup().expect("Failed to setup SFTP test harness");
    let base = harness.data_path();
    let b = &harness.backend;

    // --- display_name & is_local ---
    assert!(b.display_name().starts_with("SFTP: "));
    assert!(b.display_name().contains("127.0.0.1"));
    assert!(!b.is_local());

    // --- join_path ---
    assert_eq!(b.join_path("/home", "user"), "/home/user");
    assert_eq!(b.join_path("/home/", "user"), "/home/user");

    // --- parent_path ---
    assert_eq!(
        b.parent_path("/home/user/file.txt"),
        Some("/home/user".to_string())
    );
    assert_eq!(b.parent_path("/home"), Some("/".to_string()));
    assert_eq!(b.parent_path("/"), None);

    // --- file_name ---
    assert_eq!(
        b.file_name("/home/user/file.txt"),
        Some("file.txt".to_string())
    );
    assert_eq!(b.file_name("/dir/"), Some("dir".to_string()));

    // --- create_file ---
    let file1 = format!("{}/file1.txt", base);
    b.create_file(&file1).unwrap();
    assert!(std::path::Path::new(&file1).exists());
    assert_eq!(fs::read_to_string(&file1).unwrap(), "");

    // --- write_file + read_file ---
    let file2 = format!("{}/file2.txt", base);
    b.write_file(&file2, b"hello sftp world").unwrap();
    let data = b.read_file(&file2, 1024).unwrap();
    assert_eq!(data, b"hello sftp world");

    // --- read_file limited ---
    let data_limited = b.read_file(&file2, 5).unwrap();
    assert_eq!(data_limited, b"hello");

    // --- exists ---
    assert!(b.exists(&file1).unwrap());
    assert!(b.exists(&file2).unwrap());
    assert!(!b.exists(&format!("{}/nonexistent", base)).unwrap());

    // --- metadata file ---
    let info = b.metadata(&file2).unwrap();
    assert!(info.is_file);
    assert!(!info.is_dir);
    assert_eq!(info.size, 16); // "hello sftp world"

    // --- create_dir ---
    let dir1 = format!("{}/subdir", base);
    b.create_dir(&dir1).unwrap();
    assert!(std::path::Path::new(&dir1).is_dir());

    // --- metadata dir ---
    let dir_info = b.metadata(&dir1).unwrap();
    assert!(dir_info.is_dir);
    assert!(!dir_info.is_file);

    // --- list_dir ---
    let entries = b.list_dir(&base).unwrap();
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"file1.txt"));
    assert!(names.contains(&"file2.txt"));
    assert!(names.contains(&"subdir"));

    // --- list_dir empty ---
    let empty_entries = b.list_dir(&dir1).unwrap();
    assert_eq!(empty_entries.len(), 0);

    // --- list_dir with metadata ---
    let file_entry = entries.iter().find(|e| e.name == "file2.txt").unwrap();
    assert!(file_entry.info.is_file);
    assert_eq!(file_entry.info.size, 16);
    let dir_entry = entries.iter().find(|e| e.name == "subdir").unwrap();
    assert!(dir_entry.info.is_dir);

    // --- canonicalize ---
    let dotdot = format!("{}/subdir/..", base);
    let canonical = b.canonicalize(&dotdot).unwrap();
    let expected = fs::canonicalize(&base)
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert_eq!(canonical, expected);

    // --- rename ---
    let renamed = format!("{}/renamed.txt", base);
    b.rename(&file1, &renamed).unwrap();
    assert!(!std::path::Path::new(&file1).exists());
    assert!(std::path::Path::new(&renamed).exists());
    assert_eq!(fs::read_to_string(&renamed).unwrap(), "");

    // --- copy_file ---
    let copied = format!("{}/copied.txt", base);
    b.copy_file(&file2, &copied).unwrap();
    assert!(std::path::Path::new(&file2).exists()); // original still there
    assert_eq!(fs::read_to_string(&copied).unwrap(), "hello sftp world");

    // --- remove_file ---
    b.remove_file(&renamed).unwrap();
    assert!(!std::path::Path::new(&renamed).exists());

    // --- copy_dir ---
    let src_dir = format!("{}/src_copy", base);
    b.create_dir(&src_dir).unwrap();
    b.write_file(&format!("{}/a.txt", src_dir), b"aaa").unwrap();
    b.write_file(&format!("{}/b.txt", src_dir), b"bbb").unwrap();

    let dst_dir = format!("{}/dst_copy", base);
    b.copy_dir(&src_dir, &dst_dir).unwrap();
    assert!(std::path::Path::new(&dst_dir).is_dir());
    assert_eq!(
        fs::read_to_string(format!("{}/a.txt", dst_dir)).unwrap(),
        "aaa"
    );
    assert_eq!(
        fs::read_to_string(format!("{}/b.txt", dst_dir)).unwrap(),
        "bbb"
    );

    // --- remove_dir_all ---
    let rmdir = format!("{}/to_remove", base);
    b.create_dir(&rmdir).unwrap();
    b.write_file(&format!("{}/x.txt", rmdir), b"x").unwrap();
    b.create_dir(&format!("{}/nested", rmdir)).unwrap();
    b.write_file(&format!("{}/nested/y.txt", rmdir), b"y")
        .unwrap();

    b.remove_dir_all(&rmdir).unwrap();
    assert!(!std::path::Path::new(&rmdir).exists());

    println!("All SFTP operations passed!");
}
