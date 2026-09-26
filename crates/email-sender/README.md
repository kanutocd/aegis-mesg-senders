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
