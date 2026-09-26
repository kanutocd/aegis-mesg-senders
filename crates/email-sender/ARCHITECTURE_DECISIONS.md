# Architecture Decisions

## ADR-001: Depend on aegis-mesg-sender-core for shared delivery behavior

Status: Accepted

This crate owns email semantics only. Routing, health, circuit breaking,
attempt history, state, and observability use `aegis-mesg-sender-core` so SMS and email
behave consistently.

## ADR-002: Use HTTP adapters instead of provider SDKs

Status: Accepted

Thin `reqwest`-compatible adapters reduce dependency weight and keep request
mapping, error classification, and test injection under project control.

## ADR-003: Make providers feature-gated

Status: Accepted

Resend, Mailgun, and Mailpit integrations are optional Cargo features. Users
compile only the providers they need.

## ADR-004: Treat webhooks as a separate inbound boundary

Status: Accepted

Outbound submission and inbound delivery receipts use separate types and
validation paths. A submission response never implies final delivery.
