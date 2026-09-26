# Contributing

Read `AGENTS.md`, `ARCHITECTURE.md`, and `ARCHITECTURE_DECISIONS.md` before
changing code. Keep changes small, document public APIs, add deterministic tests,
and record user-visible changes in `CHANGELOG.md`.

Required checks:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --all-features --no-deps
```

## API and release policy

Review `API_COMPATIBILITY.md` before changing public types or traits. Public
API changes require a changelog entry and release-note update. Versioning,
breaking-change rules, and the boundary between core contracts and optional
consumer-owned integrations are documented there.
