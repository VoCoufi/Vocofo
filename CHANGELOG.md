# Changelog

All notable changes to Vocofo are documented in this file.

## [1.0.0] - 2026-03-19

### Added
- **SFTP/FTP/SCP support**: Browse and manage files on remote servers
  - SFTP backend via ssh2 with key, password, and ssh-agent auth
  - FTP backend via suppaftp with timeout and metadata parsing
  - SCP fallback when SFTP subsystem is unavailable
  - Cross-backend copy/move between local and remote panels
  - Connection dialog (F5) with protocol selection
  - Connection bookmarks (F7) saved to config (no passwords stored)
  - Auto-reconnect with 60-second keep-alive checks
  - Transfer progress indicator in status bar
- **Command palette** (F1): Searchable list of all 19 actions grouped by section
- **Settings popup** (F2): Panel layout, hidden files, preview, default path
- **chmod support** (Ctrl+M): Change file permissions with octal input
- **Panel layout options**: Auto, Horizontal, or Vertical via settings
- **Configuration file**: `~/.config/vocofo/config.toml` with atomic saves
- **File preview** (F3): File contents and directory listings in split pane
- **Search/filter** (/): Filter files by name in current directory
- **Bulk selection**: Space to toggle, Ctrl+A select all, batch copy/move/delete
- **Overwrite confirmation**: Prompt before replacing existing files
- **Sync panels** (=): Match inactive panel path to active panel

### Changed
- **Keyboard shortcuts**: Destructive actions now require Ctrl or Del key
  - Delete: `d` → `Del`
  - Rename: `r` → `Ctrl+R`
  - New file: `n` → `Ctrl+N`
  - New folder: `p` → `Ctrl+P`
- **Status bar**: Shows only F-key shortcuts; all actions via F1 palette
- **FilesystemBackend trait**: Unified abstraction for local/remote operations
- **Code structure**: event_handler and render split into focused modules

### Fixed
- Memory exhaustion when copying large files (read_file buffer sized to actual file)
- SFTP symlink detection (checks S_IFLNK bit in permission field)
- FTP connection hangs (30-second read/write timeout)
- FTP exists()/canonicalize() silently masking connection errors
- Connect dialog accepting invalid port numbers
- Config file corruption on crash (atomic write via temp + rename)
- Batch operation errors now report "X of Y failed" format

## [0.1.0] - 2025

### Added
- Dual-panel file browser with ratatui TUI
- Basic file operations: copy, paste, delete, create folder/file
- Vim-style navigation (j/k/h/l, g/G)
- Color-coded file listing (directories blue, files green)
- Confirmation dialogs for destructive operations
- Cut/move operations (Ctrl+X)
- Rename functionality
- Background thread file operations with spinner
- Responsive layout (vertical on tall terminals)
- Hidden files toggle
