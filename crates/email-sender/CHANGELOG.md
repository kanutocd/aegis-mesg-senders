# Changelog

## Unreleased

- Fixed Mailpit success handling for ID-less HTTP 200 responses by retaining
  the local request message ID as the correlation ID.
- Added ignored, credential-gated live tests for Resend, Mailgun, and Mailpit.
- Added the approved Aegis GitHub avatar to the crate README.
- Added provider response fixtures, Mailgun replay protection, application
  message-ID extraction, and Resend/Svix signature verification.

- Added Mailgun multipart attachment requests and base64 Resend attachment payloads.
- Added signed Mailgun webhook verification and provider-specific Resend/Mailgun status parsing.

- Corrected Mailgun authentication and form encoding, mapped recipient/header
  fields, and rejected attachments until multipart transport is supported.
- Add provider usage documentation, a composition example, and tag-triggered
  release validation workflow.
- Add the typed email facade, validation and composition model, injectable
  transport boundary, Resend/Mailgun/Mailpit adapters, receipt mapping, and
  deterministic protocol tests.
- Add the aegis-email-sender crate manifest, local `aegis-mesg-sender-core` dependency, and
  project security, architecture, and contribution documentation.
- Implement typed email composition, validation, receipt normalization, and
  deterministic injectable HTTP adapters for Resend, Mailgun, and Mailpit.
- Add provider documentation, a composition example, and tag-based release
  checks.
- Document harness commit and changelog rules, and the local `aegis-mesg-sender-core`
  repository path.
- Initial Rust port planning for provider-neutral email delivery.
