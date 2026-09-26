<p align="center">
  <img src="../../docs/branding/github/aegis-github-avatar.svg" alt="Aegis" width="128">
</p>

# aegis-email-sender

Typed, provider-neutral email composition and HTTP adapters backed by
[`aegis-mesg-sender-core`](../mesg-sender-core).

## Providers

Resend, Mailgun, and Mailpit are optional Cargo features and are enabled by
default. Disable the default features when only a subset is needed:

```toml
aegis-email-sender = { version = "0.1", default-features = false, features = ["resend"] }
```

Adapters accept an injected [`HttpTransport`](src/lib.rs), so applications can
use their preferred synchronous HTTP client and tests can use deterministic
fixtures. Submission returns provider acceptance; webhook receipts are mapped
separately with `normalize_receipt`.

For credential-gated live checks, run the workspace Mailpit service with
`docker compose -f ../../docker-compose.mailpit.yml up -d`, or run the ignored
Resend/Mailgun/Mailpit harness from the workspace README.
