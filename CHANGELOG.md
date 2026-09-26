# Changelog

## Unreleased

- Added the approved Aegis GitHub branding assets to every workspace README.

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
