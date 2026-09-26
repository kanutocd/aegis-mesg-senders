# Architecture Decisions

## ADR-001: Depend on aegis-mesg-sender-core for shared delivery behavior

Status: Accepted

SMS and email must share routing, error, failover, health, circuit, state, and
observability semantics. SMS-specific validation and adapters remain here.

## ADR-002: Use direct HTTP integrations

Status: Accepted

The Ruby gem uses documented HTTP APIs rather than mandatory provider SDKs.
Rust adapters will preserve that boundary and use injectable HTTP clients.

## ADR-003: Keep provider integrations feature-gated

Status: Accepted

TextBee, Semaphore, Twilio, and Infobip are optional features. Applications can
compile only the providers they configure.

## ADR-004: Preserve acceptance versus delivery semantics

Status: Accepted

Submission responses become `accepted` delivery states. Delivery receipts are
separate events and may later transition the state to delivered, failed, or
unknown.
