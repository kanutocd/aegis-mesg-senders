<p align="center">
  <img src="docs/branding/github/aegis-github-social-preview.png" alt="Aegis — The boringly reliable API gateway for every tenant." width="100%">
</p>

# aegis-mesg-senders

[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://www.rust-lang.org/)

Rust crates for channel-neutral and channel-specific outbound message
delivery.

## Project origin and purpose

These crates were originally extracted from the **Multi-Tenant Aegis API
Gateway** project, where they were first implemented as project-local crates
for the gateway's messaging needs.

They are now maintained as reusable public crates so other Rust projects can
share the same delivery contracts and provider integrations. For example,
<font size="+1">**ᜎᜓ**</font> **Lunsaran**, the multi-organization resumable uploads and workflows platform,
can use these crates without copying or reimplementing the messaging layer.

The project was intentionally moved from private, application-specific code
to a public repository and the [crates.io](https://crates.io/) registry. The
`aegis-` package prefix identifies the crates as part of this ecosystem and
helps avoid naming collisions with unrelated public packages.

## Crates

- [`aegis-mesg-sender-core`](crates/mesg-sender-core): routing,
  resilience, normalized delivery state, and safe observability.
- [`aegis-email-sender`](crates/email-sender): typed email composition
  and provider adapters.
- [`aegis-sms-sender`](crates/sms-sender): validated SMS facade and provider
  adapters; release inclusion remains an explicit operational decision.

## Development

Run checks for the complete workspace:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps

# Generate coverage and enforce the core/email thresholds
cargo +nightly llvm-cov --branch \
  --package aegis-mesg-sender-core --package aegis-email-sender \
  --all-features --json --output-path coverage.json \
  --fail-under-functions 80 --fail-under-lines 85 --fail-under-regions 85

# SMS has a separate baseline gate in CI while adapter-path coverage expands
cargo +nightly llvm-cov --branch --package aegis-sms-sender --all-features
```

Each crate remains independently publishable to crates.io.

The initial release scope is the core runtime and email sender. SMS now
has validation, provider adapters, receipts, and fixture-style transport tests;
live provider sandbox execution and operational sign-off remain prerequisites.
