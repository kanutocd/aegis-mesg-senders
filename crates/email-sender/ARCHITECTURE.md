# aegis-email-sender Architecture

## Purpose

`aegis-email-sender` ports the email consumer of the Multi-Tenant Aegis API Gateway project. 
It provides an email-oriented facade while delegating routing and resilience to
`aegis-mesg-sender-core`.

## Components

- `EmailMessage`: immutable validated sender, recipients, subject, text/HTML,
  CC, BCC, reply-to, headers, attachments, and metadata.
- `EmailProvider`: channel-specific adapter contract.
- Resend adapter: JSON API and idempotency-key support.
- Mailgun adapter: form API and sandbox-specific error mapping.
- Mailpit adapter: local development JSON API.
- Receipt normalizer: provider response and webhook mapping.

## Flow

```text
Email::send -> validate EmailMessage -> convert to core request
            -> aegis-mesg-sender-core Router -> provider adapter -> Delivery
            -> receipt/event normalization
```

Provider credentials and endpoint configuration are supplied by the host
application. Adapter HTTP clients must be injectable for tests.

## Invariants

1. Invalid addresses and missing required fields fail before election.
2. An email must contain text, HTML, or another supported body representation.
3. Provider-specific rejection classification controls failover.
4. Idempotency keys are preserved for safe retry where the provider supports it.
5. Raw email content and credentials never enter logs or generic events.
