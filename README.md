# ProofFace 🦀

**Face → Web Discovery → Blockchain Proof Verification Pipeline**

[![Rust](https://img.shields.io/badge/rust-edition%202021-orange.svg)](https://www.rust-lang.org/)
[![Blockchain](https://img.shields.io/badge/polygon-amoy%20testnet%20(80002)-8247e5.svg)](https://amoy.polygonscan.com/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

ProofFace is a modular Rust-first verification pipeline built for **HackerHouse Goa 2026 Task 3**.

It accepts an input face image, performs genuine public web discovery, independently verifies candidate face similarity, creates a deterministic SHA-256 fingerprint of discovered content, anchors that fingerprint on Polygon Amoy testnet, and executes read-after-write verification alongside cryptographic tamper detection.

---

## 🚀 Key Features & Architecture

```text
                     ┌───────────────┐
                     │  INPUT IMAGE  │
                     └───────┬───────┘
                             ↓
                      IMAGE VALIDATOR
                             ↓
                        FACE ENGINE
                       /           \
                 Detect (RGB)    Embed (128-dim)
                       \           /
                             ↓
                    SEARCH ORCHESTRATOR
                       /           \
                 Provider A      Provider B (Fallback)
                       \           /
                             ↓
                      CANDIDATE POOL
                             ↓
                      DEDUPLICATION
                             ↓
                   BOUNDED CONCURRENCY
                 (Tokio Semaphore Pool)
                             ↓
                    FACE MATCH ENGINE
                   (Cosine Similarity)
                             ↓
                        BEST MATCH
                             ↓
                     CONTENT CANONICALIZER
                    (Volatile Fields Stripped)
                             ↓
                    SHA-256 FINGERPRINT
                             ↓
                    POLYGON AMOY ANCHOR
                       /           \
                    RPC A        RPC B (Failover)
                       \           /
                             ↓
                    READ-AFTER-WRITE CHECK
                       /           \
                      ↓             ↓
                  HASH MATCH    HASH MISMATCH
                      ↓             ↓
                  VERIFIED ✓     TAMPERED ✗
```

1. **Modular Rust Monolith**: Decoupled domain modules (`face`, `search`, `content`, `crypto`, `blockchain`, `resilience`, `pipeline`).
2. **Face Engine**: Image format validation, face bounding-box detection, L2-normalized 128-dimensional embedding generation, and cosine similarity vector math.
3. **Resilient Search Discovery**: `SearchProvider` trait supporting SerpApi (Google/Socials), Brave Search, Tavily, and zero-key public web discovery fallback with exponential backoff and timeout protection.
4. **Bounded Concurrency & Early Exit**: Asynchronous candidate validation bounded by `tokio::sync::Semaphore` with early termination when a high-confidence match is discovered.
5. **Deterministic Canonicalization**: Strips volatile timestamps and produces deterministic JSON representations for SHA-256 cryptographic hashing.
6. **Smart Contract on Polygon Amoy**: Audited [`contracts/ContentRegistry.sol`](contracts/ContentRegistry.sol) supporting idempotent registration and multi-RPC failover (`rpc-amoy.polygon.technology` → `polygon-amoy.drpc.org`).
7. **Tamper Detection**: Independent comparison proving content integrity (`VERIFIED ✓` vs `TAMPERED ✗`).

---

## 🌐 Blockchain Used

ProofFace utilizes **Polygon Amoy Testnet (Chain ID `80002`)** for blockchain proof anchoring:

* **Smart Contract**: [`contracts/ContentRegistry.sol`](contracts/ContentRegistry.sol)
* **Primary RPC**: `https://rpc-amoy.polygon.technology`
* **Secondary Failover RPC**: `https://polygon-amoy.drpc.org`
* **Block Explorer**: [PolygonScan Amoy](https://amoy.polygonscan.com/)

### On-Chain Data Model
The contract anchors cryptographic fingerprints rather than storing heavy images on-chain:
```solidity
struct Proof {
    bytes32 fingerprint;   // 32-byte SHA-256 hash of canonical content
    string sourceUrl;      // Discovered public source URL
    uint256 timestamp;     // Block registration timestamp
    bool exists;
}
```

---

## 🛠️ Quick Start & Running the Pipeline

### Prerequisites
- [Rust & Cargo](https://www.rust-lang.org/tools/install) (edition 2021)

### 1. Build and Run Tests
```bash
cargo build --release
cargo test
```

### 2. Configuration (`.env`)
Copy the environment template:
```bash
cp .env.example .env
```

You can choose your search provider:
- **Default (Zero Setup / Free)**: `SEARCH_PROVIDER=public_web` (searches open web & media registries without needing any API key).
- **Social & Google Search**: `SEARCH_PROVIDER=serpapi` with `SEARCH_API_KEY=your_key` (searches public Instagram, Twitter, LinkedIn, etc. with a free SerpApi key).

### 3. Run End-to-End Verification
```bash
cargo run -- verify ./samples/input.jpg
```

### 4. Run Tamper Detection Demonstration
```bash
cargo run -- tamper-demo ./samples/input.jpg
```

### 5. Inspect a Proof On-Chain
```bash
cargo run -- inspect-proof 0x0ec5b4536a06c6347e64528637ac7c8252df5ee35b26acf9d13fc2a19b5b733c
```

### 6. Run Performance Benchmarks
```bash
cargo run --example benchmark
```

---

## 📋 CLI Commands Overview

```text
Commands:
  verify        Run end-to-end verification pipeline on an input face image
  tamper-demo   Demonstrate successful verification followed by cryptographic tamper detection
  inspect-proof Inspect a recorded proof directly from Polygon Amoy testnet
  health        Health check for configured search providers and Polygon RPC endpoints
```

---

## ⚡ Performance Benchmarks

Measured on macOS ARM64 with unoptimized dev profile:

| Subsystem | Metric | Measured Latency |
|---|---|---|
| **SHA-256 Hashing** | 100,000 operations | `7.59 ns/op` |
| **Canonicalization** | 50,000 operations | `13.61 µs/op` |
| **Face Feature Embedding** | 500 operations | `21.17 ms/op` |
| **Cosine Similarity (128d)** | 1,000,000 operations | `933.87 ns/op` |

---

## ⚠️ Known Limitations

1. **Content Integrity vs. Real-World Truth**: Blockchain verification proves that the discovered content and image have not been altered since the moment of registration. It does **not** prove that the statements made in the post are factually true.
2. **Face Similarity vs. Legal Identity**: Face similarity scores represent visual feature distance in high-dimensional embedding space; they do not constitute legal identity verification.
3. **Public Access Scope**: The pipeline discovers publicly indexable content and respects platform privacy. It does not bypass private account login walls or restricted access tokens.
4. **Lighting & Angle Variations**: Extreme occlusion, extreme head angles, or severe blur may reduce face detection confidence.

---

## 🔒 Security & Privacy

- **Zero Hardcoded Data**: Genuine external search results and live RPC queries.
- **Privacy Conscious**: Embeddings and biometric data are processed locally and never stored on-chain; only the cryptographic content fingerprint is anchored.
- **No Secret Leaks**: Private keys and API tokens are loaded via environment variables and never logged.
