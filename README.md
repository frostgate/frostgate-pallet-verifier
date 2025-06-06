# Frostgate Verifier Pallet

A Substrate pallet for verifying cross-chain messages using zero-knowledge proofs. Part of the Frostgate protocol for trustless cross-chain communication.

## Overview

The Frostgate Verifier pallet provides on-chain verification of cross-chain messages using SP1 zero-knowledge proofs. It supports:

- Message submission and verification
- Proof verification using SP1 zkVM
- Verification key management
- Program caching
- Nonce tracking per chain and account

## Installation

Add this to your runtime's `Cargo.toml`:

```toml
[dependencies]
pallet-frostgate-verifier = { version = "0.1.0", default-features = false }
```

## Configuration

Include it in your runtime:

```rust
parameter_types! {
    pub const MaxPayloadSize: u32 = 1024;
    pub const MessageDeposit: Balance = 100;
    pub const MaxKeySize: u32 = 1024 * 1024; // 1MB
    pub const MaxProgramAge: u32 = 1000; // blocks
}

impl pallet_frostgate_verifier::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type MaxPayloadSize = MaxPayloadSize;
    type MessageDeposit = MessageDeposit;
    type MaxKeySize = MaxKeySize;
    type MaxProgramAge = MaxProgramAge;
}
```

## Usage

### Submitting Messages

```rust
// Submit a message
FrostgateVerifier::submit_message(
    origin,
    ChainId::Ethereum,
    ChainId::Polkadot,
    payload,
    Some(proof),
)?;
```

### Verifying Messages

```rust
// Verify a message
FrostgateVerifier::verify_message(
    origin,
    message_hash,
)?;
```

### Managing Verification Keys

```rust
// Add a verification key (root only)
FrostgateVerifier::add_verification_key(
    root_origin,
    program_hash,
    key_bytes,
    Some(metadata),
)?;
```

### Caching Programs

```rust
// Cache a program (root only)
FrostgateVerifier::cache_program(
    root_origin,
    program_hash,
    program_bytes,
)?;
```

## Events

- `MessageSubmitted` - New message submitted
- `MessageVerified` - Message verified successfully
- `MessageVerificationFailed` - Message verification failed
- `VerificationKeyAdded` - New verification key added
- `ProgramCached` - Program cached

## Errors

- `PayloadTooLarge` - Message payload exceeds size limit
- `MessageNotFound` - Message not found
- `InvalidChainId` - Invalid chain identifier
- `InvalidProof` - Invalid proof format
- `AlreadyVerified` - Message already verified
- `InvalidStatusTransition` - Invalid message status transition
- `VerificationFailed` - Proof verification failed
- `KeyTooLarge` - Verification key exceeds size limit
- `InvalidKey` - Invalid verification key format
- `ProgramNotFound` - Program not found in cache

## Storage

- `Messages` - Maps message hash to message data
- `Nonces` - Maps (chain_id, account) to nonce
- `VerificationKeys` - Maps program hash to verification key
- `ProgramCache` - Maps program hash to program data

## License

Licensed under Apache License, Version 2.0 