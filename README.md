# ProofFace 🦀

**Face → Web Discovery → Blockchain Proof**

ProofFace is a Rust-first end-to-end verification pipeline built for HackerHouse Goa 2026 Task 3.

It accepts a face image, performs genuine public-web discovery, independently validates candidate face similarity, creates a deterministic SHA-256 fingerprint of discovered content, anchors that fingerprint on Polygon Amoy, and re-verifies the content against the on-chain record.

## Core idea

```text
Face image
   ↓
Face detection + embedding
   ↓
Real web discovery
   ↓
Candidate verification
   ↓
Content fingerprint
   ↓
Polygon Amoy
   ↓
Independent verification
   ↓
VERIFIED / TAMPERED / UNVERIFIED
```

## Engineering focus

The project is intentionally a modular Rust application rather than a microservice stack.

Key engineering concepts:

- provider abstraction
- async I/O
- bounded concurrency
- timeout and retry
- search/RPC failover
- deterministic canonicalization
- SHA-256 content fingerprints
- idempotent proof registration
- read-after-write verification
- structured errors and logs
- graceful degradation

## Important limitation

Blockchain verification proves that the same fingerprint was registered on-chain. It does **not** prove that the content itself is factually true, nor does a face similarity score prove legal identity.

The demonstration should use publicly accessible and appropriate/consented content.

## Documentation

- [PRD](PRD.md)
- [Implementation Plan](IMPLEMENTATION_PLAN.md)

## Status

Implementation in progress.
