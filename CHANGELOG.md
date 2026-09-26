# Changelog

## Unreleased

- Added the JSON content type header required by TextBee live sends.
- Included safe structured provider messages in SMS HTTP error diagnostics.
- Aligned the Rust SMS provider request contracts with the Ruby gem, including
  the Infobip `sender`/`content.text` payload shape.
- Corrected TextBee live integration payload and response handling to match its
  current API contract.
- Fixed Mailpit live delivery handling when its successful response has no
  provider message ID.
- Added an opt-in live provider test harness for Resend, Mailgun, Mailpit,
  TextBee, and Infobip, plus a pinned Docker Compose Mailpit service.
- Removed the branch coverage threshold enforcement from CI because of a
  tooling reliability issue; function, line, and region gates remain active.
- Removed branch instrumentation from the SMS coverage job because it changes
  region totals and causes false region-gate failures.
- Removed SMS fail-under thresholds from nightly CI because nightly reports
  unstable line and region totals; SMS coverage remains published as an artifact.

- Added `crates/README.md` as an index for the three publishable workspace crates.

- Added the approved Aegis GitHub branding assets to every workspace README.
- Added provider fixtures, webhook replay/signature hardening, SMS coverage
  validation, and explicit core-plus-email initial release selection.
- Documented deterministic fixture boundaries and credential-gated live sandbox
  validation.

- Implemented SMS composition, E.164/GSM/Unicode validation, Twilio/TextBee/Semaphore/Infobip adapters, receipts, and test fixtures.
- Added core receipt transitions, same-provider retry handling, concrete Redis/JSON and tracing integrations, Mailgun multipart attachments, and signed Mailgun/Resend webhook parsing.
- Added release packaging order and dependency, license, audit, and mutation-testing automation.

- Repaired workspace paths and dependency links so all crates resolve together.
- Hardened core diagnostics and corrected Mailgun request mapping.
- Marked the SMS scaffold as excluded from the initial production release.
- Added workspace CI and package verification guidance for the staged release sequence.
- Added CI LCOV coverage measurement and artifact upload for the implemented core and email crates.
- Raised coverage quality gates for functions, lines, regions, and branches.
- Set coverage thresholds at 80% functions, 85% lines, 85% regions, and 75% branches.
- Consolidated the core, email, and SMS crates into one Cargo workspace.
- Renamed the public packages to the `aegis-*` namespace.
- Documented the extraction from the Multi-Tenant Aegis API Gateway project
  and the transition to reusable public crates.io packages.
