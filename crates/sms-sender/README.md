<p align="center">
  <img src="../../docs/branding/github/aegis-github-avatar.svg" alt="Aegis" width="128">
</p>

# aegis-sms-sender

Typed, provider-neutral SMS composition and provider adapters backed by
`aegis-mesg-sender-core`.

The facade provides validated E.164 messages, GSM-7/Unicode segment rules,
normalized receipts, and injected-transport adapters for Twilio, TextBee,
Semaphore, and Infobip. Provider credentials and HTTP clients remain owned by
the consuming application. Live provider sandbox fixtures and operational
sign-off remain release prerequisites.
