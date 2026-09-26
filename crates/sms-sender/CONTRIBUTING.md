# Contributing

Read `AGENTS.md`, `ARCHITECTURE.md`, and `ARCHITECTURE_DECISIONS.md` before
changing code. Add protocol fixtures for every provider and update
`CHANGELOG.md` for public behavior or provider changes.

Required checks:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --all-features --no-deps
```
