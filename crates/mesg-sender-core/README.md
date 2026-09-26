<p align="center">
  <img src="../../docs/branding/github/aegis-github-avatar.svg" alt="Aegis" width="128">
</p>

# aegis-mesg-sender-core

`aegis-mesg-sender-core` is a channel-neutral delivery runtime for email, SMS, and other
outbound message sender crates. It contains provider contracts, capability-aware
selection, bounded failover, circuit state, normalized delivery state, and
safe structured events.

The default build has no channel SDK, network, runtime, database, or Redis
dependency. Adapters implement the public traits at the application boundary.

## Example

Run the complete in-memory flow with:

```bash
cargo run --example basic
```

## Optional integrations

The `redis` and `telemetry` features expose small adapter contracts without
adding a Redis client, serializer, or telemetry SDK. Consumer crates can wrap
their existing dependencies with `RedisStateStore` and `TelemetryObserver`.

## Status

The core API is experimental. The feature-gated integration contracts are
stable enough for consumer crates to wrap their chosen SDKs, but the public
API may still evolve before a 1.0 release.

See [API_COMPATIBILITY.md](API_COMPATIBILITY.md) for provider error semantics,
compatibility guarantees, and release policy.
