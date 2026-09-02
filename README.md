# ProofFace 🦀

> **End-to-End Visual Provenance & Integrity Pipeline**  
> *Face Input → Genuine Web Discovery → Independent Verification → Polygon Amoy Anchoring*

[![Rust](https://img.shields.io/badge/Rust-Edition%202021-DEA584.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Polygon](https://img.shields.io/badge/Blockchain-Polygon%20Amoy%20(80002)-8247e5.svg?style=flat-square&logo=polygon)](https://amoy.polygonscan.com/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Tests](https://img.shields.io/badge/Tests-22%20Passing-brightgreen.svg?style=flat-square)](tests/)

---

## The Problem & The Idea

Finding a face online is not proof of authenticity, and downloading an image is not proof that the content hasn't been tampered with after the fact.

**ProofFace** solves this in a single, high-performance Rust pipeline:
1. It takes an input face image.
2. Performs **genuine external web/social search** (no mocked or pre-selected data).
3. Downloads candidate posts and independently verifies facial similarity using L2-normalized 128-dimensional embedding vectors.
4. Deterministically canonicalizes the discovered post into a unique **32-byte SHA-256 fingerprint**.
5. Anchors the fingerprint into an audited smart contract on **Polygon Amoy**.
6. Performs read-after-write verification to guarantee on-chain integrity, instantly flagging any downstream modification as **`TAMPERED`**.

```text
 ┌─────────────┐
 │ Input Face  │
 └──────┬──────┘
        │ [1/7] Validate image format & dimensions
        ▼
 ┌─────────────┐
 │ Face Engine │ ──> Extract 128-d spatial & gradient feature vector
 └──────┬──────┘
        │ [4/7] Query Search Orchestrator (SerpApi / Brave / Public Web)
        ▼
 ┌─────────────┐
 │ Web Search  │ ──> Discover real public posts / images
 └──────┬──────┘
        │ [5/7] Bounded Concurrent Validation (Tokio Semaphore)
        ▼
 ┌─────────────┐
 │ Match Score │ ──> Independent Cosine Similarity (threshold >= 0.75)
 └──────┬──────┘
        │ [6/7] Strip volatile fields & compute deterministic SHA-256
        ▼
 ┌─────────────┐
 │ Canonicalize│ ──> 32-Byte Content Fingerprint (0x...)
 └──────┬──────┘
        │ [7/7] Anchor on Polygon Amoy & Read-After-Write Verification
        ▼
 ┌─────────────┐
 │ Blockchain  │ ──> Polygon Amoy (Chain ID 80002) with RPC Failover
 └──────┬──────┘
        │
   ┌────┴───────────────────────────┐
   ▼                                ▼
VERIFIED ✓                      TAMPERED ✗
(Exact On-Chain Hash Match)    (Content Altered Post-Registration)
```

---

## System Architecture & Engineering Highlights

ProofFace is built as a **modular Rust monolith** designed for maximum resilience, type safety, and zero bloat:

* **Pure Rust Face Engine**: Custom bounding-box segmentation, skin-tone isolation, and 128-dimensional spatial-frequency feature extraction.
* **Resilient Multi-Provider Search**: Plug-and-play `SearchProvider` trait supporting **SerpApi (Google/Instagram/X)**, **Brave Search**, **Tavily**, and a zero-key **Public Web discovery fallback** with exponential backoff and timeout handling.
* **Bounded Concurrency & Early Exit**: Workers are throttled via `tokio::sync::Semaphore` to prevent rate-limit exhaustion. If a worker hits a high-confidence match (`>= 0.75`), remaining pending candidate tasks terminate early to save network bandwidth and compute.
* **Deterministic Canonicalization**: Strips volatile timestamps and sorts JSON keys to ensure identical logical content always produces the exact same SHA-256 digest across different runs.
* **Polygon Amoy Smart Contract**: Minimal Solidity contract [`contracts/ContentRegistry.sol`](contracts/ContentRegistry.sol) supporting idempotent registration and automated primary/secondary RPC failover.

---

## Terminal Experience

### 1. End-to-End Verification (`cargo run -- verify ./samples/input.jpg`)

```text
╔══════════════════════════════════════════════════════════╗
║                      PROOFFACE 🦀                        ║
║     Face → Web Discovery → Blockchain Proof              ║
╚══════════════════════════════════════════════════════════╝

[1/7] Validating image... ✓ Valid image (6348 bytes)
[2/7] Detecting face... ✓ 1 face detected (confidence: 0.65)
[3/7] Generating embedding... ✓ L2-normalized 128-dim embedding generated
[4/7] Searching public web for candidates (query: "input profile photo")...
      ✓ 10 search candidate URLs discovered
[5/7] Verifying candidates (bounded concurrency: 5)...
      #Candidate 01 ........ similarity: 0.412 (NoMatch)
      #Candidate 02 ........ similarity: 0.574 (PossibleMatch)
      #Candidate 06 ........ similarity: 0.969 (HighConfidence)

      ★ MATCH CONFIRMED (similarity: 0.969)
      Source: https://en.wikipedia.org/wiki/UFRaw
      Media:  https://upload.wikimedia.org/wikipedia/commons/thumb/8/89/UFRaw-0-13-screenshot.png/330px-UFRaw-0-13-screenshot.png
[6/7] Creating deterministic SHA-256 fingerprint... ✓ Fingerprint: 0x0ec5b4536a06c6347e64528637ac7c8252df5ee35b26acf9d13fc2a19b5b733c
      Anchoring on Polygon Amoy (Chain ID 80002)... ✓ Confirmed
      Tx Hash: 0x16eaed3db0c16ec565a68c4082bfbd72a394fe049c2f3c31c2bc9e55215bc7d4
[7/7] Re-verifying against on-chain record... ✓ Match confirmed

╔══════════════════════════════════════════════════════════╗
║                      VERIFIED ✓                          ║
╚══════════════════════════════════════════════════════════╝
```

### 2. Tamper Detection Demo (`cargo run -- tamper-demo ./samples/input.jpg`)

```text
--- [SIMULATING UNAUTHORIZED CONTENT MODIFICATION] ---
Simulating alteration of title/media metadata on discovered post...
Registered On-Chain Fingerprint : 0x0ec5b4536a06c6347e64528637ac7c8252df5ee35b26acf9d13fc2a19b5b733c
Recalculated Tampered Fingerprint: 0xe625aa2da06e76dc00672045b6aa6db2d01e840017a4397c805ecff5adf5cda9
Comparison Result               : MISMATCH ✗

╔══════════════════════════════════════════════════════════╗
║                      TAMPERED ✗                          ║
║          ProofFace detected altered content              ║
╚══════════════════════════════════════════════════════════╝
```

---

## Getting Started

### Prerequisites
* Rust toolchain (1.75+ / edition 2021)

```bash
# Clone the repository
git clone https://github.com/nikhilkumarpanigrahi/proofFace.git
cd proofFace

# Run unit & integration test suite (22 tests)
cargo test
```

### Configuration (`.env`)

Copy the template:
```bash
cp .env.example .env
```

| Variable | Default | Description |
|---|---|---|
| `SEARCH_PROVIDER` | `public_web` | `public_web` (free/zero-key), `serpapi`, `brave`, or `tavily` |
| `SEARCH_API_KEY` | *None* | API key for commercial search providers |
| `RPC_PRIMARY` | `https://rpc-amoy.polygon.technology` | Primary Polygon Amoy RPC endpoint |
| `RPC_SECONDARY` | `https://polygon-amoy.drpc.org` | Secondary failover RPC endpoint |
| `CHAIN_ID` | `80002` | Polygon Amoy network chain ID |
| `MAX_CONCURRENT_CANDIDATES`| `5` | Maximum parallel worker threads |

---

## CLI Reference

```bash
# 1. Run full verification pipeline on a face image
cargo run -- verify ./samples/input.jpg

# 2. Run verification followed by live tamper demonstration
cargo run -- tamper-demo ./samples/input.jpg

# 3. Query on-chain record for a specific 32-byte fingerprint
cargo run -- inspect-proof 0x0ec5b4536a06c6347e64528637ac7c8252df5ee35b26acf9d13fc2a19b5b733c

# 4. Check configuration & RPC network connectivity
cargo run -- health

# 5. Run performance benchmark suite
cargo run --example benchmark
```

---

## ⚡ Measured Benchmarks

Measured on macOS ARM64:

```text
• SHA-256 Hashing          : 7.59 ns/op   (100,000 iterations)
• Canonicalization         : 13.61 µs/op  (50,000 iterations)
• Face Feature Embedding   : 21.17 ms/op  (500 iterations)
• Cosine Similarity (128d) : 933.87 ns/op (1,000,000 iterations)
```

---

## 🔗 Smart Contract (Polygon Amoy)

The contract [`contracts/ContentRegistry.sol`](contracts/ContentRegistry.sol) is intentionally lightweight. Instead of polluting the chain with heavy raw images, it anchors 32-byte SHA-256 digests:

```solidity
struct Proof {
    bytes32 fingerprint;   // 32-byte SHA-256 hash of canonical content
    string sourceUrl;      // Discovered public source URL
    uint256 timestamp;     // Block registration timestamp
    bool exists;
}
```

---

## ⚠️ Important Limitations

* **Integrity ≠ Factual Truth**: ProofFace cryptographically proves that discovered content has not been altered since the moment of registration. It does **not** prove that statements made in a public post are factually accurate.
* **Similarity ≠ Legal Identity**: Facial similarity is a distance calculation across a 128-dimensional embedding space; it indicates visual feature correlation, not government-issued identity proof.
* **Public Boundary**: The pipeline operates exclusively on publicly accessible, search-indexable web endpoints and does not bypass authenticated walls or private social media accounts.

---

## License

MIT License. Built for HackerHouse Goa 2026.
