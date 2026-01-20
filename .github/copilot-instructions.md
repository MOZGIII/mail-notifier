# Copilot / AI agent instructions

Purpose: give AI coding agents the minimal, actionable context to be productive in this repository.

- **Project layout (big picture):** This is a Rust workspace. CLI binaries live under `crates/bin/*`; shared library code lives under `crates/lib/*` (for example the IMAP logic in `crates/lib/imap-checker`). The top-level Cargo.toml defines the workspace and CI/packaging expects the workspace layout.

- **Primary binary:** See [crates/bin/main/src/main.rs](../crates/bin/main/src/main.rs) — binaries are intentionally thin: keep business logic in the library crates and call into them from `main`.

- **Key library crate:** See [crates/lib/imap-checker](../crates/lib/imap-checker) for core email-checking logic. Prefer to add/modify logic here rather than in the `bin` crate.

- **Build / test / run commands:**
  - Build whole workspace: `cargo build --workspace`
  - Build release binary: `cargo build -p main --release` or `cd crates/bin/main && cargo build --release`
  - Run binary locally: `cargo run -p main` or `cargo run --bin main` from workspace root
  - Run tests: `cargo test --workspace`

- **Toolchain & linting choices:**
  - Toolchain is pinned in `rust-toolchain.toml` (nightly) and CI expects it.
  - Workspace uses a strict lint profile via `.cargo/config.toml` (notably `-Dunsafe_code`, `-Wmissing_docs`, and clippy warnings); keep new code lint-clean.
  - Rustdoc is built with `--document-private-items`.

- **Dependency management:**
  - Prefer adding crates to `[workspace.dependencies]` in the root `Cargo.toml`, then reference them with `workspace = true` in member crates.
  - Prefer the latest stable crate versions by default unless a compatibility reason is documented.
  - Crates use edition 2024; keep new crates aligned.

- **Repo hygiene tools:**
  - `deny.toml` configures `cargo-deny`; keep license choices and advisory ignores consistent.
  - This repo uses `cargo shear` to detect unused dependencies; keep changes compatible with it.
  - `taplo.toml` defines TOML formatting (key order is enforced for `Cargo.toml`).
  - `typos.toml` configures spelling checks.

- **Docker / packaging:** The repo contains a `Dockerfile` and `docker-bake.hcl`. CI uses those for producing images — do not assume local Docker is required for small code changes.

- **Repository scripts & helpers:** `build-utils/` contains helper scripts used by CI and releases (for example `list-bin-targets` and `archive-binaries`). Use these when adding new binaries or release artifacts.

- **CI and automation:** See the `.github/workflows/` directory for GitHub Actions used by this project. Keep changes that affect build, linting, or artifact layout in sync with workflow definitions.

- **Code conventions & patterns discovered in repo:**
  - Core logic belongs to library crates under `crates/lib/*` so it can be tested independently and reused by multiple binaries.
  - Binary crates under `crates/bin/*` should be thin: argument parsing, configuration, and invocation of library functions.
  - Prefer explicit, domain-specific error types in library crates; avoid using `anyhow`/`eyre` in library code. Use `thiserror` or custom `enum` error types so callers can match and handle errors precisely.

- **Where to add tests:** Place unit tests alongside library modules in `crates/lib/.../src` and integration tests in the `tests/` directory of the crate if needed.

- **What to avoid / not assume:**
  - Do not modify CI/workflow files without updating `.github/workflows/*` and `build-utils/` where appropriate.
  - Do not move core logic into `bin` crates; keep it in `crates/lib/*`.

- **Examples (actionable edits):**
  - To add a feature that checks a new IMAP flag, implement logic in [crates/lib/imap-checker/src](../crates/lib/imap-checker/src) and add unit tests there, then call it from [crates/bin/main/src/main.rs](../crates/bin/main/src/main.rs).
  - To add a new CLI binary, create `crates/bin/<name>/Cargo.toml` and `src/main.rs`, then update `build-utils/list-bin-targets` if it enumerates binaries.

- **Quick debugging tips:**
  - Use `RUST_LOG=debug cargo run -p main` to enable debug logging where supported.
  - Run `cargo test -p imap-checker -- --nocapture` to see test output in failing cases.

If anything above is unclear or you want more examples drawn from specific files, say which area (build, a crate, or CI) and I will expand this file.
