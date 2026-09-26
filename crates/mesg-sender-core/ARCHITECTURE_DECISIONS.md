# Architecture Decisions

## ADR-001: Keep the core channel-neutral

Status: Accepted

The core exposes provider-neutral contracts. Email and SMS validation, message
composition, provider catalogs, and response mapping remain in their channel
crates. This preserves reuse without weakening channel-specific type safety.

## ADR-002: Use traits for providers and state

Status: Accepted

Provider adapters, state persistence, clocks, and observers are traits. Tests
can use deterministic fakes, while production applications can add HTTP,
Redis, or telemetry integrations without coupling the core to them.

## ADR-003: Make resilience policy explicit

Status: Accepted

Retryability, failover eligibility, circuit thresholds, cooldowns, and attempt
limits are configuration and typed policy, not hidden loops in adapters.

## ADR-004: Separate accepted from delivered

Status: Accepted

Provider APIs usually acknowledge submission before final delivery. Delivery
events update normalized state later and must retain provider correlation IDs.

## ADR-005: Avoid mandatory async/runtime dependencies in the model layer

Status: Accepted

Transport and async dependencies belong behind optional integrations. Core value
objects and policies should remain easy to test and embed.
