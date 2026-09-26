# aegis-sms-sender Architecture

## Purpose

`aegis-sms-sender` ports the SMS consumer of the Multi-Tenant Aegis API Gateway project.
It provides a typed SMS facade and provider adapters while delegating common delivery policy
to `aegis-mesg-sender-core`.

## Components

- `SmsMessage`: immutable validated E.164 recipients, body, sender, Unicode
  requirements, metadata, and provider options.
- `SmsProvider`: channel-specific adapter contract.
- TextBee adapter: API key, device selection, and batch acceptance mapping.
- Semaphore adapter: Philippine SMS API mapping and rejection handling.
- Twilio adapter: account credentials, sender or messaging service, and SID
  mapping.
- Infobip adapter: base URL, API key, sender, and bulk message mapping.
- Receipt normalizer: provider status callbacks and message IDs.

## Flow

```text
Sms::send -> validate SmsMessage -> convert to core request
          -> aegis-mesg-sender-core Router -> provider adapter -> Delivery
          -> receipt/event normalization
```

## Invariants

1. Recipients are validated before provider election.
2. Invalid sender or body input is terminal and cannot trigger failover.
3. SMS length and Unicode requirements are explicit; no silent truncation.
4. Provider acceptance does not confirm handset delivery.
5. Phone numbers, message text, and credentials are excluded from logs/events.
6. Named provider instances have independent health and circuit state.
