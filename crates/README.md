<p align="center">
  <img src="../docs/branding/github/aegis-github-avatar.svg" alt="Aegis" width="128">
</p>

# Aegis crates

This directory contains the three publishable crates in the Aegis message
sender workspace:

| Crate | Purpose | Documentation |
| --- | --- | --- |
| `mesg-sender-core` | Channel-neutral routing, resilience, delivery state, and observability. | [`README.md`](mesg-sender-core/README.md) |
| `email-sender` | Typed email composition and Resend, Mailgun, and Mailpit adapters. | [`README.md`](email-sender/README.md) |
| `sms-sender` | Validated SMS composition and Twilio, TextBee, Semaphore, and Infobip adapters. | [`README.md`](sms-sender/README.md) |

The core crate is the dependency foundation. Channel-specific crates build on
its provider and delivery contracts while retaining their own validation and
provider mappings.
