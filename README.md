# Vocofo

[![Crates.io](https://img.shields.io/crates/v/vocofo.svg)](https://crates.io/crates/vocofo)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](#)

A fast, lightweight terminal-based file manager written in Rust. Navigate your filesystem efficiently with keyboard-driven controls and a clean, color-coded interface.

## Demo

<!-- TODO: Add screenshot or animated GIF showing Vocofo in action -->
*Screenshots and demo GIF coming soon!*

## Features

- **Lightning Fast**: Built with Rust for maximum performance and safety
- **Keyboard-Driven**: Complete control without touching your mouse
- **Intuitive Navigation**: Vim-like controls for familiar workflows
- **File Operations**: Copy, paste, delete, and create with simple shortcuts
- **Visual Feedback**: Color-coded files and folders, highlighted selection
- **Confirmation Dialogs**: Safety checks before destructive operations
- **Cross-Platform**: Works on Linux, macOS, and Windows

## Quick Start

```bash
# Clone the repository
git clone https://gitlab.com/Coufik/Vocofo.git
cd Vocofo

# Build and run
cargo run

# Or install locally
cargo install --path .
vocofo
```

## Installation

### Build from Source

**Prerequisites:**
- Rust 1.70+ (install from [rustup.rs](https://rustup.rs/))

**Steps:**
```bash
git clone https://gitlab.com/Coufik/Vocofo.git
cd Vocofo
cargo build --release
```

The compiled binary will be in `target/release/vocofo`.

### Binary Releases

Pre-built binaries for Linux, macOS, and Windows will be available in future releases. Watch this space!

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑` / `↓` | Move selection up/down |
| `Enter` | Open selected folder or file |
| `Tab` | Navigate to parent directory |
| `P` | Create new folder (popup dialog) |
| `D` | Delete selected file/folder (with confirmation) |
| `Ctrl+C` | Copy selected file/folder to clipboard |
| `Ctrl+V` | Paste from clipboard to current directory |
| `Esc` / `Q` | Exit application |

## Usage

### Basic Navigation

1. **Launch Vocofo** in your terminal
2. Use `↑` and `↓` arrows to select files/folders
3. Press `Enter` to open the selected folder
4. Press `Tab` to go back to the parent directory
5. Press `Q` or `Esc` to quit

### Creating Folders

1. Press `P` to open the "Create Folder" dialog
2. Type the folder name
3. Press `Enter` to create, or `Esc` to cancel

### Copying and Pasting Files

1. Navigate to the file or folder you want to copy
2. Press `Ctrl+C` to copy it to the clipboard
3. Navigate to the destination folder
4. Press `Ctrl+V` to paste

**Note:** If a file with the same name exists at the destination, the operation will fail (overwrite protection).

### Deleting Files

1. Navigate to the file or folder you want to delete
2. Press `D` to open the confirmation dialog
3. Press `Enter` to confirm deletion, or `Esc` to cancel

## Architecture

Vocofo follows a clean, modular architecture with clear separation of concerns:

- **State Machine Pattern**: Different UI modes (Normal, CreatePopup, ConfirmDelete)
- **Event-Driven**: Keyboard events routed to appropriate handlers
- **Centralized State**: All application state managed through the `Context` struct
- **Filesystem Isolation**: All file operations in dedicated module

## Testing

Vocofo has a comprehensive test suite with **48 tests** covering:
- File operations (create, delete, copy)
- State management and navigation
- Copy/paste workflows
- Edge cases and error handling

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test file
cargo test --test file_operations_test
```

## Contributing

Contributions are welcome! Here's how you can help:

1. **Fork** the repository
2. **Create a feature branch**: `git checkout -b feature/amazing-feature`
3. **Make your changes** and add tests
4. **Run the test suite**: `cargo test`
5. **Commit your changes**: `git commit -m 'Add amazing feature'`
6. **Push to the branch**: `git push origin feature/amazing-feature`
7. **Open a Merge Request**

### Development Guidelines

- Follow Rust best practices and idioms
- Add tests for new features
- Use `PathBuf` for cross-platform path handling
- Avoid `.unwrap()` and `.expect()` in user-facing code
- Document public functions with doc comments

### Code Style

```bash
# Format code
cargo fmt

# Run linter
cargo clippy

# Check for errors
cargo check
```

## Roadmap

Planned features and enhancements:

- **Overwrite Confirmation**: Popup dialog when pasting over existing files
- **Cut/Move Operations**: Ctrl+X for cut, move files between directories
- **Visual Clipboard Indicator**: Show what's currently copied in status bar
- **Progress Bars**: For large file/folder operations
- **Search Functionality**: Quick file search within current directory
- **Bookmarks**: Save and jump to frequently-used directories
- **File Preview**: View file contents in a split pane
- **Bulk Operations**: Select multiple files for batch operations

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

Built with excellent open-source libraries:
- [ratatui](https://github.com/ratatui-org/ratatui) - Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Cross-platform terminal manipulation
- [edit](https://crates.io/crates/edit) - Text editor integration

Special thanks to the Rust community for their invaluable resources and support.

---

**Project Status**: Active development (v0.1.0)

For questions, issues, or suggestions, please open an issue on [GitLab](https://gitlab.com/Coufik/Vocofo/-/issues).
