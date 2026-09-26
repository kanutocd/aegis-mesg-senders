#!/usr/bin/env bash
set -euo pipefail

cargo package -p aegis-mesg-sender-core --allow-dirty
cargo package -p aegis-email-sender --allow-dirty
cargo package -p aegis-sms-sender --allow-dirty
