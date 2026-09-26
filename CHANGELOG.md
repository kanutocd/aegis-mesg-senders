# Changelog

## Unreleased

- Repaired workspace paths and dependency links so all crates resolve together.
- Hardened core diagnostics and corrected Mailgun request mapping.
- Marked the SMS scaffold as excluded from the initial production release.
- Added workspace CI and package verification guidance for the staged release sequence.
- Added CI LCOV coverage measurement and artifact upload for the implemented core and email crates.
- Consolidated the core, email, and SMS crates into one Cargo workspace.
- Renamed the public packages to the `aegis-*` namespace.
- Documented the extraction from the Multi-Tenant Aegis API Gateway project
  and the transition to reusable public crates.io packages.
