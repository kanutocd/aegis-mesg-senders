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
**Lunsaran**, the multi-organization resumable uploads and workflows project,
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
- [`aegis-sms-sender`](crates/sms-sender): planned SMS facade scaffold. It is
  included for workspace development but is not production-ready or part of
  the initial provider release.

## Development

Run checks for the complete workspace:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
```

Each crate remains independently publishable to crates.io.

The initial release scope is the core runtime and email sender. SMS remains a
documented scaffold until its validation, provider adapters, receipts, and
fixture-driven tests are implemented.
