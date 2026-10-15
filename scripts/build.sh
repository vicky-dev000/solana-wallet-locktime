#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if [[ "$(solana-verify --version)" != "solana-verify 0.5.2" ]]; then
  echo "This build requires solana-verify 0.5.2." >&2
  exit 1
fi

solana-verify build \
  --library-name sol_wallet_locktime \
  --base-image solanafoundation/solana-verifiable-build@sha256:a20e03cbd0b96bf3d4084e956cf6d7b47389c7a35508628a82296c3376518be9 \
  --cargo-build-sbf-args='--tools-version v1.48'

solana-verify get-executable-hash target/deploy/sol_wallet_locktime.so
