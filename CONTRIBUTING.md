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
cargo +nightly llvm-cov --branch \
  --package aegis-mesg-sender-core --package aegis-email-sender \
  --all-features --json --output-path coverage.json \
  --fail-under-functions 80 --fail-under-lines 85 --fail-under-regions 85
jq -e '.data[0].totals.branches.count > 0 and .data[0].totals.branches.percent >= 75' coverage.json

# Verify each independently publishable package
cargo package -p aegis-mesg-sender-core
cargo package -p aegis-email-sender
cargo package -p aegis-sms-sender
```

Coverage currently targets the implemented core and email crates. The SMS
crate is excluded until its scaffold has production code and tests.

Coverage gates are 80% functions, 85% lines, 85% regions, and 75% branches.
Branch coverage requires a nightly Rust toolchain because Rust branch
instrumentation remains unstable.
