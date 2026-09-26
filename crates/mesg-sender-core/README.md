<p align="center">
  <img src="../../docs/branding/github/aegis-github-avatar.svg" alt="Aegis" width="128">
</p>

# aegis-mesg-sender-core

`aegis-mesg-sender-core` is a channel-neutral delivery runtime for email, SMS, and other
outbound message sender crates. It contains provider contracts, capability-aware
selection, bounded failover, circuit state, normalized delivery state, and
safe structured events.

The default build has no channel SDK, network, runtime, database, or Redis
dependency. The opt-in `redis` feature provides a synchronous Redis client and
JSON codec; the opt-in `telemetry` feature provides a safe `tracing` observer.
Applications can also implement the public traits at their own boundary.

## Example

Run the complete in-memory flow with:

```bash
cargo run --example basic
```

## Optional integrations

The `redis` feature exposes `RedisClientBackend`, `JsonDeliveryCodec`, and
`RedisStateStore`. The `telemetry` feature exposes `TracingObserver` and the
generic `TelemetryObserver` for application-owned sinks.

## Status

The core API is experimental. The feature-gated integration contracts are
stable enough for consumer crates to wrap their chosen SDKs, but the public
API may still evolve before a 1.0 release.

See [API_COMPATIBILITY.md](API_COMPATIBILITY.md) for provider error semantics,
compatibility guarantees, and release policy.
