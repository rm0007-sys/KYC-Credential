# 🔐 KYC Credential — Soroban Smart Contract

> **On-chain identity verification for the Stellar network.**  
> Issue, query, and revoke tamper-proof KYC credentials directly on-chain using Soroban smart contracts.

---

## 📋 Project Description

**KYC Credential** is a Soroban smart contract deployed on the Stellar blockchain that acts as a decentralised KYC (Know Your Customer) registry. A trusted admin (e.g. a regulated identity provider or compliance team) issues verifiable credentials to wallet addresses after completing off-chain identity checks. Any other contract or dApp can then query the registry in real time to gate access, features, or funds — without ever touching the underlying personal data.

Personal data never touches the blockchain. Only the *result* of verification (level + expiry + an opaque reference hash) is stored on-chain, keeping the system both privacy-respecting and audit-friendly.

---

## ⚙️ What It Does

| Action | Who | Description |
|--------|-----|-------------|
| `initialize` | Admin | Deploy and set the controlling admin address (one-time) |
| `issue` | Admin | Mint a KYC credential for a verified wallet address |
| `is_verified` | Anyone | Returns `true` if credential exists, is active, and not expired |
| `meets_level` | Anyone | Returns `true` if credential meets a minimum KYC tier |
| `get_credential` | Anyone | Fetch the raw credential record for inspection |
| `update` | Admin | Upgrade level, extend expiry, or link a new report reference |
| `revoke` | Admin | Mark a credential as inactive (audit trail preserved) |
| `delete` | Admin | Permanently erase a record (GDPR right-to-erasure) |
| `transfer_admin` | Admin | Hand off admin role to a new address |

---

## ✨ Features

### 🪪 Three-Tier Verification Levels
```
Basic    →  Name + date-of-birth check
Standard →  Full document verification (passport / national ID)
Enhanced →  AML screening + enhanced due-diligence
```
Other contracts can gate access by *minimum* level — e.g. require `Standard` for withdrawals above $1 000.

### ⏳ Time-Bound Credentials
Every credential can carry an `expires_at` UNIX timestamp. After expiry, `is_verified` automatically returns `false` — no manual cleanup needed. Pass `0` to create a non-expiring credential.

### 🔗 Off-Chain Reference Anchoring
Each credential stores a `reference` field (a hash, UUID, or IPFS CID) that points to the full KYC report kept in a compliant off-chain data store. On-chain data stays minimal; the reference provides an auditable link back to the source-of-truth.

### 🔒 Admin-Gated Writes, Public Reads
Only the admin (a multisig, DAO, or compliance service) can issue, update, revoke, or delete credentials. Any wallet or contract can call read functions (`is_verified`, `meets_level`, `get_credential`) without authentication — making composability frictionless.

### ♻️ Revocation Without Erasure
`revoke` marks a credential inactive while keeping the record on-chain for audit purposes. `delete` performs a hard-erase for right-to-erasure compliance.

### 🧪 Full Test Suite
The contract ships with unit tests covering:
- Happy-path issuance + verification
- Expiry enforcement via ledger timestamp manipulation
- Level-gating logic
- Duplicate issuance prevention
- Double-initialisation protection

---

## 🏗️ Project Structure

```
kyc-credential/
├── Cargo.toml          # Soroban SDK dependency & release optimisation profile
└── src/
    └── lib.rs          # Contract code, types, errors, and tests
```

---

## 🚀 Getting Started

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add the WASM target
rustup target add wasm32-unknown-unknown

# Install Soroban CLI
cargo install --locked soroban-cli
```

### Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

The compiled `.wasm` lives at:
```
target/wasm32-unknown-unknown/release/kyc_credential.wasm
```

### Run Tests

```bash
cargo test
```

### Deploy to Testnet

```bash
# Configure testnet identity
soroban keys generate --global mykey --network testnet

# Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/kyc_credential.wasm \
  --source mykey \
  --network testnet
```

### Invoke on Testnet

```bash
# Initialise (replace CONTRACT_ID and ADMIN_ADDRESS)
soroban contract invoke \
  --id CONTRACT_ID --source mykey --network testnet \
  -- initialize --admin ADMIN_ADDRESS

# Issue a credential
soroban contract invoke \
  --id CONTRACT_ID --source mykey --network testnet \
  -- issue \
    --owner USER_ADDRESS \
    --level Standard \
    --expires_at 0 \
    --reference "sha256:abc123..."

# Check if verified
soroban contract invoke \
  --id CONTRACT_ID --network testnet \
  -- is_verified --owner USER_ADDRESS

# Check level
soroban contract invoke \
  --id CONTRACT_ID --network testnet \
  -- meets_level --owner USER_ADDRESS --required Enhanced
```

---

## 🔄 Integration Example

Any Soroban contract can gate access using this registry:

```rust
// In your DeFi / dApp contract
let kyc_verified: bool = kyc_client.is_verified(&caller);
if !kyc_verified {
    return Err(MyError::KycRequired);
}

// Or enforce a minimum tier
let cleared: bool = kyc_client.meets_level(&caller, &KycLevel::Standard);
if !cleared {
    return Err(MyError::InsufficientKycLevel);
}
```

---

## 🔐 Security Considerations

- **Admin key security** — use a multisig or hardware-backed key for the admin address in production.
- **No PII on-chain** — only store hashes or opaque IDs in the `reference` field, never raw personal data.
- **Expiry hygiene** — always set `expires_at` for credentials that must be re-verified periodically (e.g. annual AML refresh).
- **Credential uniqueness** — the contract prevents re-issuance without an explicit `update` call, avoiding duplicate credential exploits.

---

## 📄 License

MIT — free to use, modify, and deploy.

---

## 🤝 Built With

- [Soroban SDK](https://developers.stellar.org/docs/smart-contracts) — Stellar's smart contract platform
- [Rust](https://www.rust-lang.org/) — systems language powering Soroban contracts
- [Stellar Testnet](https://developers.stellar.org/docs/fundamentals-and-concepts/testnet-and-pubnet) — for development & testing



wallet address: GBMSXBWNB2AX6DORIERMRTLIWMQ2SSBCQCUVCWGRT5P5QERLPZBWPSLS

contract address: CDW7SOGFMHQDOVPRML7MKASULZPXPTH3N2UOM7THIAR6HATB4GURZLHR

https://stellar.expert/explorer/testnet/contract/CDW7SOGFMHQDOVPRML7MKASULZPXPTH3N2UOM7THIAR6HATB4GURZLHR

<img width="1600" height="900" alt="image" src="https://github.com/user-attachments/assets/d452749d-bc7e-499e-9da0-e89414d088c6" />

