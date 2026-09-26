# aegis-mesg-sender-core Architecture

## Purpose

`aegis-mesg-sender-core` is the channel-neutral runtime for reusable outbound message delivery
crates. It is currently consumed by `aegis-email-sender` and `aegis-sms-sender`.

## Dependency direction

```text
channel facade -> channel message/adapter -> aegis-mesg-sender-core contracts/router
                                          -> transport/provider implementation
```

`aegis-mesg-sender-core` must not depend on either channel crate.

## Main boundaries

- `Message` and channel-neutral metadata define the routing input.
- `Provider` defines a provider adapter contract and capabilities.
- `ProviderRegistry` owns named provider instances and configuration.
- `Router` performs eligibility checks, election, bounded failover, and result
  normalization.
- `Error` classification decides retry and failover behavior.
- `CircuitBreaker` and `Health` maintain provider runtime state.
- `StateStore` abstracts memory, Redis, or another shared backend.
- `Observer` emits structured events without sensitive payloads.

## Invariants

1. Invalid messages fail before provider election.
2. Terminal errors never trigger provider failover.
3. Retry and failover attempts are explicitly bounded.
4. Provider acceptance is not equivalent to handset or mailbox delivery.
5. Provider credentials remain in configuration and never in delivery events.
6. Named provider instances have independent priority, health, and circuit state.

## Runtime flow

```text
validate -> select eligible providers -> attempt primary
         -> classify result -> record health/circuit/event
         -> bounded failover -> normalized Delivery
```
