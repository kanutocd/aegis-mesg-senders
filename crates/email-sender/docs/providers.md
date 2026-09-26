# Provider adapters

Each adapter implements `aegis_mesg_sender_core::Provider` and accepts an `Arc<dyn
HttpTransport>`. The application supplies the concrete HTTP client; tests can
provide a fixture transport without credentials or network access.

| Adapter | Endpoint shape | Authentication |
| --- | --- | --- |
| Resend | `{endpoint}/emails` | Bearer API key |
| Mailgun | `{endpoint}/{domain}/messages` | Basic API key |
| Mailpit | `{endpoint}/api/v1/send` | None |

Provider responses are only submission acceptance. Delivery confirmation is
represented separately by `DeliveryReceipt` and `normalize_receipt`.

Deterministic response fixtures are stored under `fixtures/` and exercised by
unit tests. Live provider checks require application-owned HTTP transports and
credentials, so they are intentionally kept out of normal CI.
