# Solana Wallet Locktime

Source code and pinned build inputs for public verification of a Solana program.

## On-chain addresses

- Program ID: `T95WDuuLJAyMtESkf9adxKxLZqP36cUxfb6TEUhLock`
- ProgramData: `3ivfKxa5XbwF1CWzFhEsr11uSGmNCuizf5gBhctPpfYm`
- Network: Solana Mainnet Beta

## Reproducible build

The contract uses a root-level Cargo package:

```text
.cargo/config.toml
Cargo.toml
Cargo.lock
src/lib.rs
scripts/build.sh
target/deploy/sol_wallet_locktime.so  (generated)
```

Requirements: Git, Bash, Docker with Linux amd64 support, and the official
[`solana-verify`](https://github.com/solana-foundation/solana-verifiable-build)
CLI, version `0.5.2`. ARM hosts require Docker's amd64 emulation.

```bash
cargo install solana-verify --version 0.5.2 --locked
git clone https://github.com/vicky-dev000/solana-wallet-locktime.git
cd solana-wallet-locktime
# Check out the exact commit selected for verification before building.
bash scripts/build.sh
```

The output is `target/deploy/sol_wallet_locktime.so`. The single Cargo package
at the repository root exports `sol_wallet_locktime`. `Cargo.lock` pins the dependency
graph, and `solana-verify` builds with `--locked`. Keep the manifest and source
layout unchanged: source paths embedded in the executable affect its hash.

See [VERIFICATION.md](VERIFICATION.md) for the image digest, compiler version,
artifact fingerprint, and commit-pinned verification command.

## Scope

This repository contains only contract source and verification inputs. Building
does not deploy or upgrade a program, publish verification metadata, or establish
that the result matches the current on-chain executable. No signing key is
needed for a local build.
