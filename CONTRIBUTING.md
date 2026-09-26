# Contributing

Read `AGENTS.md` and the crate-specific documentation before changing code.
Keep channel-specific behavior in channel crates and shared routing,
resilience, state, and observability behavior in
`aegis-mesg-sender-core`.

Required checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
cargo llvm-cov --package aegis-mesg-sender-core --package aegis-email-sender \
  --all-features --lcov --output-path lcov.info

# Verify each independently publishable package
cargo package -p aegis-mesg-sender-core
cargo package -p aegis-email-sender
cargo package -p aegis-sms-sender
```

Coverage currently targets the implemented core and email crates. The SMS
crate is excluded until its scaffold has production code and tests.
