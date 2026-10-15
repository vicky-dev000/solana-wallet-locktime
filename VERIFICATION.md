# Program verification

## Build

Requirements: Docker with `linux/amd64` support and `solana-verify 0.5.2`.
The build uses the digest-pinned official image `2.2.1`, platform-tools
`v1.48` (SBF Rust `1.84.1-dev`, Cargo `1.84.0`), and the committed `Cargo.lock`.

Run from the repository root:

```bash
bash scripts/build.sh
solana-verify get-executable-hash target/deploy/sol_wallet_locktime.so
```

## Expected artifact

A clean-target rebuild reproduced the following local artifact:

| Check | Value |
| --- | --- |
| Size | `98856` bytes |
| Executable hash | `456180fc9f8da292efb83d2ea386d3aaaa3982dcbfdb2f2d83872e8b4b11cd8b` |
| File SHA-256 | `ac860e76192e64897fdf208e1aaf0ad95230a3833a8a417b6b50dc9c2cd687cb` |

Use the executable hash for on-chain comparisons: it excludes trailing zero
padding. These fingerprints do not imply deployment or remote verification.

## Verify a public commit

Use a clean checkout of a commit already available in the public repository.
Read the deployed hash first; it must match the local executable hash:

```bash
solana-verify --url https://api.mainnet-beta.solana.com get-program-hash \
  T95WDuuLJAyMtESkf9adxKxLZqP36cUxfb6TEUhLock
```

Build and compare that exact commit with the same configuration:

```bash
COMMIT=$(git rev-parse HEAD)
solana-verify --url https://api.mainnet-beta.solana.com verify-from-repo \
  https://github.com/vicky-dev000/solana-wallet-locktime \
  --commit-hash "$COMMIT" \
  --program-id T95WDuuLJAyMtESkf9adxKxLZqP36cUxfb6TEUhLock \
  --library-name sol_wallet_locktime \
  --base-image solanafoundation/solana-verifiable-build@sha256:a20e03cbd0b96bf3d4084e956cf6d7b47389c7a35508628a82296c3376518be9 \
  --cargo-build-sbf-args='--tools-version v1.48'
```

`verify-from-repo` may prompt to sign and publish verification metadata after
a successful comparison. Only an authorized signer should approve this step.

After metadata publication, request remote verification (replace the placeholder):

```bash
solana-verify remote submit-job \
  --program-id T95WDuuLJAyMtESkf9adxKxLZqP36cUxfb6TEUhLock \
  --uploader <VERIFICATION_METADATA_SIGNER>
```

Check the remote result separately; an upgrade or local hash match does not
automatically establish verified status.
