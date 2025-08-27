# Repository Guidelines

## Project Structure & Modules
- Root: Cargo workspace in `Cargo.toml`.
- `axum-mcp/`: core library (`src/{lib,layer,registry,tool,http,security,prelude}.rs`).
- `axum-mcp-macros/`: proc-macro crate (`src/lib.rs`) providing `#[mcp_tool]`.
- `examples/demo/`: runnable HTTP demo (`src/main.rs`).
- `docs/`: design + plans (`docs/goals.md`, `docs/plans/plan-http-mvp.md`).
- Tests: integration tests in `axum-mcp/tests/`, unit tests inline via `#[cfg(test)]`.

## Build, Test, and Run
- `cargo build --workspace`: compile all crates.
- `cargo test -p axum-mcp`: run library tests.
- `cargo run -p axum-mcp-demo`: start demo at `127.0.0.1:37650`.
- `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings`: format and lint (CI gates).
- `cargo doc --workspace --no-deps`: generate local API docs.

## Coding Style & Naming
- Indentation: 4 spaces; follow `rustfmt` defaults.
- Naming: `snake_case` (functions/modules), `CamelCase` (types/traits), `SCREAMING_SNAKE_CASE` (consts).
- Visibility: prefer `pub(crate)` unless a public API is intentional.
- Errors: use `thiserror`-backed enums; return `Result<T, E>` with precise variants.
- Features: `http` (default), `stdio` (future). Keep `cfg(feature = ...)` tidy.

## Testing Guidelines
- Unit tests: colocated in source files.
- Integration: `axum-mcp/tests/*.rs` (e.g., `http_mvp.rs`).
- Naming: behavior focused (e.g., `it_rejects_missing_version_header`).
- Coverage focus: registry insert/call, JSON error mapping, header/origin checks.

## Commit & PR Guidelines
- Commits: Conventional Commits.
  - Examples: `feat(layer): route POST /mcp`, `fix(security): tighten origin check`, `docs: expand HTTP MVP plan`.
- PRs must include: purpose, linked issues, testing notes (commands + expected JSON), and doc updates when user-facing.
- Pre-merge checks: `cargo fmt`, `cargo clippy -D warnings`, `cargo test` all green.

## Security & Configuration Tips
- Default bind to `127.0.0.1`; validate `MCP-Protocol-Version: 2025-06-18` and local `Origin`.
- Limit CORS to local development; avoid exposing `/mcp` on public interfaces.
- Log minimally; no secrets/PII in errors.

## Architecture Overview
- Axum `McpLayer` intercepts `POST /mcp` and dispatches to a thread-safe `ToolRegistry`.
- Tools are declared with `#[mcp_tool]` and collected via `linkme`; schemas via `schemars`.
- See `docs/goals.md` and `docs/plans/plan-http-mvp.md` for milestones and design intent.
