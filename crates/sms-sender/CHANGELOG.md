# Changelog

## Unreleased

- Aligned Infobip request construction with the Ruby adapter and added
  TextBee/Infobip request-shape regression coverage.
- Corrected TextBee live requests and response correlation to match its
  `recipients`, optional `deviceId`, and `data.smsBatchId` API contract.
- Preserved normalized HTTP status details in the opt-in live test diagnostics.
- Added ignored, credential-gated live tests for TextBee and Infobip.
- Added the approved Aegis GitHub avatar to the crate README.

- Implemented the SMS domain model, E.164/GSM/Unicode/segment validation, metadata, receipts, and injected HTTP adapters for Twilio, TextBee, Semaphore, and Infobip.
- Added response fixtures for all SMS providers, provider-shaped ID parsing,
  and percent-encoded form requests.
- Documented credential-gated live sandbox validation and fixture ownership.
