# Vocofo

[![Crates.io](https://img.shields.io/crates/v/vocofo.svg)](https://crates.io/crates/vocofo)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A fast, lightweight terminal-based file manager written in Rust with dual-panel layout, vim-style navigation, and remote filesystem support (SFTP/FTP/SCP).

## Features

- **Dual-Panel Layout**: Side-by-side or stacked panels (auto/horizontal/vertical)
- **Vim-Style Navigation**: `j/k/h/l`, `g/G`, search with `/`
- **File Operations**: Copy, cut, paste, delete, rename, chmod — all with Ctrl+ shortcuts to prevent accidents
- **Remote Filesystems**: SFTP, FTP, and SCP fallback with connection bookmarks
- **Command Palette**: `F1` opens searchable list of all available actions
- **Settings UI**: `F2` for panel layout, hidden files, preview, default path
- **File Preview**: `F3` toggle preview panel with file contents/directory listing
- **Bulk Selection**: Space to select, `Ctrl+A` select all, batch operations
- **Progress Tracking**: Transfer progress shown for cross-backend file copies
- **Reconnect**: Automatic keep-alive and reconnection for remote connections
- **Configurable**: Settings persisted to `~/.config/vocofo/config.toml`

## Quick Start

```bash
cargo install vocofo
vocofo
```

> **Note:** SFTP support requires `libssh2` and `openssl` system libraries. See [Installation](#from-source) for details. To install without remote support: `cargo install vocofo --no-default-features`

## Installation

### From crates.io

```bash
cargo install vocofo
```

### From Source

**Prerequisites:**
- Rust 1.85+ ([rustup.rs](https://rustup.rs/))
- `libssh2` and `openssl` dev libraries (for SFTP support)

```bash
# Debian/Ubuntu
sudo apt install libssh2-1-dev libssl-dev pkg-config

# Fedora
sudo dnf install libssh2-devel openssl-devel

# Arch
sudo pacman -S libssh2 openssl

# macOS (via Homebrew)
brew install libssh2 openssl
```

```bash
git clone https://github.com/VoCoufi/Vocofo.git
cd Vocofo
cargo install --path .
```

### Feature Flags

SFTP and FTP are enabled by default. To build without remote support:

```bash
cargo build --release --no-default-features
```

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `↑/↓` or `j/k` | Move selection |
| `Enter` or `l` | Open file/folder |
| `Backspace` or `h` | Parent directory |
| `Tab` | Switch panel |
| `g g` | Go to first item |
| `G` | Go to last item |
| `PageUp/PageDown` | Page scroll |
| `/` | Search / filter |
| `.` | Toggle hidden files |
| `=` | Sync panels |

### File Operations

| Key | Action |
|-----|--------|
| `Ctrl+C` | Copy |
| `Ctrl+X` | Cut |
| `Ctrl+V` | Paste |
| `Del` | Delete (with confirmation) |
| `Ctrl+R` | Rename |
| `Ctrl+N` | New file |
| `Ctrl+P` | New folder |
| `Ctrl+M` | chmod (change permissions) |
| `Space` | Toggle selection |
| `Ctrl+A` | Select all |
| `Ctrl+D` | Deselect all |

### Application

| Key | Action |
|-----|--------|
| `F1` | Command palette (all actions) |
| `F2` | Settings |
| `F3` | Toggle preview |
| `F5` | Connect to remote server |
| `F6` | Disconnect |
| `F7` | Bookmarks |
| `Q` / `Esc` | Quit |

## Remote Connections

Vocofo supports browsing remote filesystems via SFTP, FTP, and SCP:

1. Press `F5` to open the connection dialog
2. Select protocol (SFTP/FTP), enter host, port, username, password
3. Press `Enter` to connect — the active panel switches to the remote filesystem
4. All file operations work — copy/move files between local and remote panels
5. Press `F6` to disconnect

### Bookmarks

- `Ctrl+S` in the connection dialog saves the current connection as a bookmark
- `F7` opens the bookmark list to quickly reconnect (passwords are never saved)

### SCP Fallback

When an SSH server doesn't have the SFTP subsystem enabled, Vocofo automatically falls back to SCP mode using SSH exec commands for browsing and SCP for file transfers.

## Configuration

Settings are stored in `~/.config/vocofo/config.toml`:

```toml
[general]
show_hidden = false
default_path = "."
show_preview_on_start = false
panel_layout = "auto"  # "auto", "horizontal", or "vertical"

[[connections]]
name = "My Server"
protocol = "sftp"
host = "example.com"
port = 22
username = "user"
```

## Testing

270 tests covering file operations, UI state, remote backends, and integration workflows:

```bash
cargo test                              # Run all tests
cargo test --test sftp_integration_test -- --ignored  # SFTP integration (needs local sshd)
```

## Architecture

```
src/
├── main.rs              # Entry point, terminal setup, main loop
├── context.rs           # Central app state (Context, PanelState, UiState)
├── backend.rs           # FilesystemBackend trait
├── local_backend.rs     # Local filesystem implementation
├── sftp_backend.rs      # SFTP via ssh2
├── ftp_backend.rs       # FTP via suppaftp
├── scp_backend.rs       # SCP fallback (SSH exec + SCP)
├── config.rs            # Config loading/saving (TOML)
├── file_operation.rs    # File ops, preview, paste resolution
├── background_op.rs     # Background threads for copy/move/delete
├── event_handler/       # Keyboard input handling (6 modules)
├── render/              # UI rendering (panels + popups)
├── ui.rs                # Layout, status bar, popup routing
└── messages_enum.rs     # UI text constants
```

## Contributing

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make changes and add tests
4. Run `cargo test && cargo clippy && cargo fmt --check`
5. Commit and open a Merge Request

## License

MIT License — see [LICENSE](LICENSE) for details.

Built with [ratatui](https://github.com/ratatui-org/ratatui), [crossterm](https://github.com/crossterm-rs/crossterm), [ssh2](https://crates.io/crates/ssh2), and [suppaftp](https://crates.io/crates/suppaftp).

---

For issues and suggestions: [GitHub Issues](https://github.com/VoCoufi/Vocofo/issues)
