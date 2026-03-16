# Vocofo - Terminal File Manager

## Project Overview
A fast, lightweight terminal-based file manager written in Rust with keyboard-driven controls and a clean TUI interface.

## Architecture
- `src/main.rs` — Entry point, terminal setup/teardown, main event loop (50ms poll)
- `src/context.rs` — Central application state (`Context` struct, `UiState` enum)
- `src/event_handler.rs` — Keyboard input handling for all UI states
- `src/file_operation.rs` — File/directory operations (list, copy, delete, preview)
- `src/ui.rs` — UI layout and component rendering
- `src/render.rs` — Advanced rendering: panels, popups, dialogs
- `src/messages_enum.rs` — Centralized UI text/message constants
- `src/lib.rs` — Library exports for testing

## Key Dependencies
- `ratatui` 0.29.0 — TUI framework
- `crossterm` 0.29.0 — Terminal manipulation
- `edit` 0.1.5 — Text editor integration

## Commands
- `cargo run` — Run the file manager
- `cargo test` — Run all tests (48+ tests across 3 test files)
- `cargo clippy` — Lint check
- `cargo fmt` — Format code
- `cargo build --release` — Release build

## Testing
Tests are in `tests/` directory:
- `context_test.rs` — Context state management tests
- `file_operations_test.rs` — File operation tests
- `copy_paste_integration_test.rs` — Integration tests

Run a specific test file: `cargo test --test file_operations_test`

## Code Style
- Use `Result` types with descriptive error messages (see `file_operation.rs` as reference)
- Avoid `unwrap()` — use `expect()` with context or proper error handling
- Named constants instead of magic numbers

## Current Status (v0.1.0)
Working toward 1.0.0 release. See `1.0-release-plan.md` for details.
