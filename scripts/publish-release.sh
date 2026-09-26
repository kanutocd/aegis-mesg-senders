#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" != "--publish" ]]; then
  echo "dry run: core -> email -> sms"
  cargo publish -p aegis-mesg-sender-core --dry-run
  cargo publish -p aegis-email-sender --dry-run
  cargo publish -p aegis-sms-sender --dry-run
  exit 0
fi

cargo publish -p aegis-mesg-sender-core
echo "Core published. Wait for crates.io indexing before publishing dependents."
read -r -p "Publish email and SMS now? [y/N] " confirmation
[[ "$confirmation" == "y" || "$confirmation" == "Y" ]]
cargo publish -p aegis-email-sender
cargo publish -p aegis-sms-sender
