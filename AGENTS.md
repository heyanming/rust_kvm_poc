# Repository Guidelines

## Project Structure & Module Organization
- `core/`: Shared types, framing, and (de)serialization used by all crates.
- `kvm_server/`: TCP listener + event injection (Enigo).
- `kvm_client/`: Local input capture (rdev) + TCP sender.
- `Cargo.toml`: Workspace root listing members. No tests directory yet.

## Build, Test, and Development Commands
- Build workspace: `cargo build` (from repo root).
- Run server: `cargo run -p kvm_server -- --listen 0.0.0.0:50051`.
- Run client: `cargo run -p kvm_client -- --connect <server_ip>:50051`.
- Debug logging: add `--debug` to either binary for verbose stderr output.
- Format: `cargo fmt --all`. Lint (optional but encouraged): `cargo clippy --all-targets -- -D warnings`.

## Coding Style & Naming Conventions
- Rust 2021 edition; 4‑space indentation; keep lines readable (<100 cols preferred).
- Names: modules/files `snake_case`, types/traits `CamelCase`, functions/vars `snake_case`.
- Public API in `core/`: keep enums and structs minimal, serializable (`serde`) and cross‑crate friendly.
- Error handling: use `anyhow::Result` in binaries; avoid panics in runtime paths.

## Testing Guidelines
- Framework: Rust `#[test]` and `tokio::test` for async.
- Start with `core/`: add unit tests alongside code (e.g., `core/src/lib.rs`) and integration tests under `core/tests/` for wire framing with `bincode`.
- Naming: use `mod tests { ... }` blocks; file names `*_test.rs` for larger suites.
- Run tests: `cargo test` or scoped `cargo test -p kvm_core`.

## Commit & Pull Request Guidelines
- Commit style: prefer Conventional Commits (`feat:`, `fix:`, `chore:`). Keep subjects imperative and <= 72 chars.
- PRs must include: purpose/summary, how to run/verify, platforms tested (macOS/Windows/Linux), and any follow‑ups.
- Link related issues; include logs or short clips if behavior changes.

## Security & OS Permissions
- Input capture/injection requires OS privileges: macOS needs Accessibility and Input Monitoring; Windows may require Administrator; Linux may need udev/X11/Wayland access.
- Network: defaults to TCP port `50051`. Avoid exposing publicly; prefer LAN or SSH tunnels.
- Do not log sensitive input. Use `--debug` only for development and never in production environments.
