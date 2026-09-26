# Release notes

## 0.1.0 — 2026-09-26

Initial experimental release of `aegis-mesg-sender-core`.

- Channel-neutral messages, provider contracts, capabilities, and normalized
  delivery state.
- Capability-aware election with priority, health weighting, stable ties, and
  explicit provider selection.
- Bounded failover with structured attempt history.
- Injectable-clock circuit breakers and rolling health tracking.
- In-memory state and observer implementations.
- Feature-gated Redis and telemetry adapter contracts that keep external SDKs
  outside the default build.
- Redacted recipients and credential-safe provider configuration diagnostics.
- Explicit provider error policy helpers and documented pre-1.0 compatibility
  and release policy.

The API is experimental. Optional adapters should be validated against the
consumer application's Redis client, serializer, and telemetry stack before
production use.
