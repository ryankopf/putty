# putty

A fast, minimal SSH host selector TUI for Windows (and others), written in Rust.

Easily browse and edit your ~/.ssh/config file using arrow keys. Select a host to immediately open an SSH session — no need to remember hostnames or flags.

## Features

- Terminal-based UI with keyboard navigation
- Reads your existing ~/.ssh/config file
- Supports editing host entries
- Optional # Password lines (not used for auth)
- Windows-compatible, with permission fixing (icacls) for key files

## Install

cargo install putty

## Usage

putty

- Navigate with ↑ / ↓
- Press Enter to connect to the selected host
- Press e to edit a host entry
- Press k to fix keyfile permissions (Windows only)
- Press q to quit

## About the Name

This project is inspired by, but entirely separate from, the original PuTTY SSH client. That project is great, but quite old — and nowadays most users connect via VS Code, Windows Terminal, or tools like Kitty. This crate offers a lightweight, memorable alternative using a clean terminal UI.

## Notes

- The app does not send passwords; # Password lines are purely for display or scripting.
- SSH keys must be in proper format (.pem or OpenSSH, not .ppk).
- Ensure your ~/.ssh/config is writable.

## License

MIT
