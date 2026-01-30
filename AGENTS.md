# AI agent instructions

Purpose: give AI coding agents the minimal, actionable context to be productive in this repository.

- **Project layout (big picture):** This is a Rust workspace. CLI binaries live under `crates/bin/*`; shared library code lives under `crates/lib/*`. The top-level Cargo.toml defines the workspace and CI/packaging expects the workspace layout. Binaries are intentionally thin: keep business logic in the library crates and call into them from `main`.

- **Build / test / run commands:**
  - Build whole workspace: `cargo build --workspace`
  - Build release binary: `cargo build -p <binary-name> --release` or `cd crates/bin/<binary-name> && cargo build --release`
  - Run binary locally: `cargo run -p <binary-name>` or `cargo run --bin <binary-name>` from workspace root
  - Run tests: `cargo test --workspace`

- **Toolchain & linting choices:**
  - Toolchain is pinned in `rust-toolchain.toml` (nightly) and CI expects it.
  - Workspace uses a strict lint profile via `.cargo/config.toml` (notably `-Dunsafe_code`, `-Wmissing_docs`, and clippy warnings); keep new code lint-clean.

- **Dependency management:**
  - Prefer adding crates to `[workspace.dependencies]` in the root `Cargo.toml`, then reference them with `workspace = true` in member crates.
  - Prefer the latest stable crate versions by default unless a compatibility reason is documented.
  - Crates use edition 2024; keep new crates aligned.
  - Use `cargo deny` for license and security checks; see `deny.toml` for configuration.

- **Code conventions & patterns discovered in repo:**
  - Core logic belongs to library crates under `crates/lib/*` so it can be tested independently and reused by multiple binaries.
  - Binary crates under `crates/bin/*` should be thin: argument parsing, configuration, and invocation of library functions.
  - Prefer explicit, domain-specific error types in library crates; avoid using `anyhow`/`eyre` in library code. Use `thiserror` or custom `enum` error types so callers can match and handle errors precisely.

- **Where to add tests:** Place unit tests alongside library modules in `crates/lib/.../src` and integration tests in the `tests/` directory of the crate if needed.

- **What to avoid / not assume:**
  - Do not modify CI/workflow files without updating `.github/workflows/*` and `build-utils/` where appropriate.
  - Do not move core logic into `bin` crates; keep it in `crates/lib/*`.
