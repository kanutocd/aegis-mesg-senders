# Changelog

## Unreleased

- Redacted raw message bodies and provider error details from debug output.
- Added coverage tests for the feature-gated Redis and telemetry contracts.
- Renamed the package and crate to `aegis-mesg-sender-core` / `aegis_mesg_sender_core`,
  updated the public repository metadata, and switched the project license to
  MIT.
- Initial Rust port planning for the channel-neutral delivery runtime.
- Added the channel-neutral architecture, architecture decisions, contribution
  guidance, and security guidance.
- Added the dependency-free crate manifest, conservative MSRV, strict lint
  configuration, README, and build-artifact exclusions.
- Implemented provider contracts, capability-aware election, explicit provider
  selection, bounded failover, attempt history, circuit and rolling health
  state, injectable clocks, in-memory persistence and observers, redacted
  events, and deterministic tests.
- Added feature-gated Redis and telemetry adapter contracts, a runnable basic
  example, release notes, and expanded invariant/concurrency coverage.
- Stabilized provider error policy helpers, documented compatibility guarantees
  and release policy, and added regression coverage for normalized semantics.
