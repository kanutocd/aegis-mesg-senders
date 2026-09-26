# API compatibility and release policy

## Compatibility guarantees

The default feature set remains dependency-free and channel-neutral. Public
traits are synchronous, `Send + Sync` where stated, and do not require an
async runtime. Provider adapters own transport-specific mapping; the core only
consumes normalized results and explicit retry/failover policy.

Provider error semantics are stable:

- `retryable` controls another attempt against the same provider.
- `failover` controls attempting another provider.
- `is_terminal()` is true only when both are false.
- `ErrorKind::default_retryable()` and `default_failover()` provide the
  baseline policy for `ProviderError::from_kind()`.
- `ProviderError::new()` remains available for provider-specific overrides.

Delivery acceptance remains distinct from final delivery. `Accepted` means
the provider accepted the submission; `Delivered` and `Failed` represent later
normalized events.

Credentials, raw message bodies, and full recipients are not part of delivery
events. Provider configuration debug output redacts credentials.

## Versioning

The crate is currently `0.x`, so breaking public API changes may occur in a
minor version and must be called out in release notes and the changelog. Patch
releases are reserved for compatible fixes and documentation corrections.

After `1.0`, breaking changes require a major version. New public APIs and
backwards-compatible behavior are minor releases; fixes are patch releases.

Feature-gated integration contracts do not add SDK dependencies. Concrete
Redis and telemetry clients remain consumer-owned until a separate integration
crate is released.

Every release must pass formatting, Clippy with warnings denied, tests,
documentation, and package verification.
