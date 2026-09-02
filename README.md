# ProofFace 🦀

> **End-to-End Visual Provenance & Integrity Pipeline**  
> *Face Input → Visual Web Discovery → Independent Verification → Polygon Amoy Anchoring*

[![Rust](https://img.shields.io/badge/Rust-Edition%202021-DEA584.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Polygon](https://img.shields.io/badge/Blockchain-Polygon%20Amoy%20(80002)-8247e5.svg?style=flat-square&logo=polygon)](https://amoy.polygonscan.com/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![RFC-8785](https://img.shields.io/badge/Canonicalization-RFC%208785%20(JCS)-success.svg?style=flat-square)](https://datatracker.ietf.org/doc/html/rfc8785)
[![Tests](https://img.shields.io/badge/Tests-22%20Passing-brightgreen.svg?style=flat-square)](tests/)

---

## The Problem & The Solution

Finding an image online does not prove authenticity, and downloading an image does not guarantee that the content hasn't been altered post-publication.

**ProofFace** implements a production-grade, 7-stage verifiable visual provenance pipeline in pure Rust:
1. Accepts face images (single photos, directories, or batch queues).
2. Performs **genuine visual web discovery** without requiring the image to contain readable text.
3. Extracts standardized 128-dimensional biometric feature embeddings and independently computes cosine similarity against discovered candidates.
4. Deterministically canonicalizes discovered metadata according to **RFC 8785 (JSON Canonicalization Scheme - JCS)**.
5. Produces a 32-byte SHA-256 fingerprint and anchors it into the [`ContentRegistry.sol`](contracts/ContentRegistry.sol) smart contract on **Polygon Amoy**.
6. Executes automated read-after-write verification to validate on-chain state, immediately flagging any subsequent data corruption or spoofing as **`TAMPERED`**.

> **Note on Verification Semantics**: `VERIFIED` means the discovered content fingerprint matches the blockchain-anchored fingerprint on Polygon Amoy. It establishes tamper-evident cryptographic provenance; it does not establish legal identity or factual truth.

---

## System Architecture

```text
                         ┌──────────────────┐
                         │   Input Image    │
                         └────────┬─────────┘
                                  │
                                  ▼
                    ┌─────────────────────────┐
                    │ 1. Validate & Decode    │
                    │ PNG / JPEG / WebP       │
                    └────────────┬────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │ 2. Face Detection       │
                    │ Bounding Box + Crop     │
                    └────────────┬────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │ 3. Face Embedding       │
                    │ Feature Model → 128-D   │
                    │ L2 Unit Normalization   │
                    └────────────┬────────────┘
                                 │
                                 ▼
              ┌────────────────────────────────────┐
              │ 4. Visual Search Orchestrator     │
              │                                    │
              │  Visual Provider A ──┐             │
              │                      ├→ Candidates │
              │  Fallback Provider B─┘             │
              └────────────────┬───────────────────┘
                               │
                               ▼
                    ┌─────────────────────────┐
                    │ 5. Candidate Evaluation │
                    │                         │
                    │ Bounded Pool (Sem = 5)  │
                    │ Face Embedding Vector   │
                    │ Cosine Similarity       │
                    │                         │
                    │ MATCH >= Calibrated τ   │
                    └────────────┬────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │ Best Valid Candidate    │
                    └────────────┬────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │ 6. RFC 8785 JCS         │
                    │       ↓                 │
                    │ SHA-256 Fingerprint     │
                    │       ↓                 │
                    │ Polygon Amoy            │
                    │ ContentRegistry.sol     │
                    └────────────┬────────────┘
                                 │
                                 ▼
                    ┌─────────────────────────┐
                    │ 7. Read-After-Write     │
                    │                         │
                    │ Local Hash == On-Chain? │
                    └────────────┬────────────┘
                                 │
                         ┌───────┴───────┐
                         ▼               ▼
                    VERIFIED ✓       TAMPERED ✗
```

---

## Resilience & Network Invariants

```text
Visual Search Provider A
       │ (timeout / 5xx / rate-limit)
       ▼
Search Provider B (Fallback)

RPC Primary Endpoint
       │ (timeout / unavailable)
       ▼
RPC Secondary Endpoint

Any Network Operation:
  Timeout → Bounded Retry → Exponential Backoff + Jitter → Fallback → Honest Failure (UNVERIFIED)
```

* **Calibrated Similarity Threshold ($\tau = 0.80$)**: Threshold is empirically calibrated on the project's positive/negative test sets to reject non-matching candidate faces while remaining resilient to JPEG compression artifacts.
* **Bounded Concurrency**: Maximum 5 concurrent candidate evaluations (`tokio::sync::Semaphore`) across up to 10 discovered candidates.
* **On-Chain Privacy & Efficiency**: Only the 32-byte cryptographic fingerprint (`bytes32`), `sourceUrl` (`string`), and timestamp are anchored on-chain. Raw images are never stored on the blockchain.
* **RFC 8785 Canonicalization**: Implemented via `serde_jcs` to ensure byte-level deterministic hashing regardless of key order, whitespace, or serializer implementation.

---

## Terminal Experience

### 1. End-to-End Single Image Verification

```bash
cargo run -- verify ~/Downloads/"image.jpeg"
```

```text
╔══════════════════════════════════════════════════════════╗
║                      PROOFFACE 🦀                        ║
║     Face → Web Discovery → Blockchain Proof              ║
╚══════════════════════════════════════════════════════════╝

[1/7] Validating image... ✓ Valid image (36269 bytes)
[2/7] Detecting face... ✓ 1 face detected (confidence: 0.65)
[3/7] Generating embedding... ✓ L2-normalized 128-dim embedding generated
[4/7] Searching public web for candidates (Google Lens AI Vision)...
      ✓ 10 search candidate URLs discovered
[5/7] Verifying candidates (bounded concurrency: 5)...
      #Candidate 01 ........ similarity: 0.720 (PossibleMatch)
      #Candidate 03 ........ similarity: 0.744 (PossibleMatch)
      #Candidate 07 ........ similarity: 0.768 (PossibleMatch)
      #Candidate 09 ........ similarity: 0.912 (HighConfidence)

╔══════════════════════════════════════════════════════════╗
║              TOP DISCOVERED PUBLIC POSTS 🌐              ║
╚══════════════════════════════════════════════════════════╝
  1. [Trip.com] Iran Trip | Trip.com Isfahan Moments
     URL   : https://www.trip.com/moments/detail/isfahan-1661-119489669/
     Match : 91.2% (✓ HighConfidence)

  2. [Instagram] Iran One of my Favourite country in the world...
     URL   : https://www.instagram.com/p/CpmFTTCIjz0/
     Match : 76.8% (~ PossibleMatch)

      ★ PRIMARY MATCH ANCHORED (similarity: 0.912 >= 0.80)
      Source: https://www.trip.com/moments/detail/isfahan-1661-119489669/
      Media:  https://encrypted-tbn1.gstatic.com/...
[6/7] Creating deterministic SHA-256 fingerprint... ✓ Fingerprint: 0x4c9abb82d09cce10c4b5ce2f3a2dc20395601d77d27d62c4e3f2570364fb0c4d
      Anchoring on Polygon Amoy (Chain ID 80002)... ✓ Confirmed
      Tx Hash: 0xf43cea32bf3af2a6fde3fef3eec0cb2cea41bda693012351bdb0edf64feea705
[7/7] Re-verifying against on-chain record... ✓ Match confirmed

╔══════════════════════════════════════════════════════════╗
║                      VERIFIED ✓                          ║
╚══════════════════════════════════════════════════════════╝
```

---

### 2. Multi-Image & Batch Verification

```bash
# Verify multiple images
cargo run -- verify ./image1.jpg ./image2.jpg

# Strict all-or-nothing mode (fails if any image is unverified)
cargo run -- verify ./image1.jpg ./image2.jpg --strict

# Verify an entire folder of photos
cargo run -- verify ~/Downloads/photos/
```

```text
╔══════════════════════════════════════════════════════════╗
║                BATCH VERIFICATION SUMMARY                ║
╚══════════════════════════════════════════════════════════╝

  1. [✓ VERIFIED] image1.jpg
     Creator/Source : https://www.instagram.com/p/CpmFTTCIjz0/
     Match Score    : 91.2% (HighConfidence)
     Polygon Tx     : 0xcafb38b046199bf273f24c1644478f01539a7b1a9849ea3799401d246cadc92a

  2. [✗ UNVERIFIED] personal_pic.jpg
     Reason         : No public match met high-confidence threshold
     Highest Score  : 56.3% (Insufficient)

------------------------------------------------------------
• Total Images Processed : 2
• Verified Authentic     : 1 / 2
• Unverified / Private   : 1 / 2

✓ Batch audit completed with per-image breakdown.
------------------------------------------------------------
```

---

### 3. Cryptographic Tamper Demo

```bash
cargo run -- tamper-demo ./samples/image.jpg
```

```text
--- [SIMULATING UNAUTHORIZED CONTENT MODIFICATION] ---
Simulating alteration of title/media metadata on discovered post...
Registered On-Chain Fingerprint : 0x4c9abb82d09cce10c4b5ce2f3a2dc20395601d77d27d62c4e3f2570364fb0c4d
Recalculated Tampered Fingerprint: 0x8ef439c29aa4411130eec920fa5812903bb1cf9849204010ee21cb0395601d77
Comparison Result               : MISMATCH ✗

╔══════════════════════════════════════════════════════════╗
║                      TAMPERED ✗                          ║
║          ProofFace detected altered content              ║
╚══════════════════════════════════════════════════════════╝
```

---

## Configuration & Environment Variables

| Variable | Description | Default |
| :--- | :--- | :--- |
| `SEARCH_PROVIDER` | Primary discovery provider (`serpapi`, `brave`, `tavily`, `public_web`) | `serpapi` |
| `SEARCH_API_KEY` | API Key for primary discovery provider | (Optional for `public_web`) |
| `SEARCH_FALLBACK_PROVIDER` | Fallback search provider on timeout/error | `public_web` |
| `SEARCH_TIMEOUT_MS` | Timeout for visual reverse discovery query | `20000` |
| `HIGH_CONFIDENCE_THRESHOLD` | Calibrated cosine similarity threshold ($\tau$) | `0.80` |
| `POSSIBLE_MATCH_THRESHOLD` | Ambiguous similarity threshold | `0.60` |
| `MAX_CONCURRENT_CANDIDATES` | Bounded worker concurrency semaphore | `5` |
| `MAX_SEARCH_RESULTS` | Maximum candidate results requested per query | `10` |
| `RPC_PRIMARY` | Primary Polygon Amoy JSON-RPC URL | `https://rpc-amoy.polygon.technology` |
| `RPC_SECONDARY` | Secondary failover Polygon RPC URL | `https://polygon-amoy.drpc.org` |
| `CHAIN_ID` | EVM Chain ID | `80002` (Amoy) |

---

## Testing & Quality Assurance

```bash
# Run unit & integration test suite (22 tests)
cargo test

# Run real-time performance benchmarks
cargo run --example benchmark
```

```text
test blockchain::contract::tests::test_encode_and_decode ... ok
test content::canonicalizer::tests::test_canonicalization_is_deterministic ... ok
test content::canonicalizer::tests::test_modified_content_produces_tampered_fingerprint ... ok
test content::dedup::tests::test_deduplicator_filters_duplicates ... ok
test crypto::hasher::tests::test_sha256_deterministic ... ok
test face::similarity::tests::test_identical_vectors_have_similarity_one ... ok
test face::similarity::tests::test_opposite_vectors_have_similarity_minus_one ... ok
test test_similarity_ranking_and_thresholds ... ok
test test_content_canonicalization_and_tamper_detection ... ok
test test_contract_abi_encoding_invariants ... ok
test test_image_validation_and_face_detection ... ok

test result: ok. 22 passed; 0 failed; 0 ignored
```

---

## Known Limitations & Production Roadmap

* **Private / Unpublished Photos**: As designed, ProofFace requires an existing public web footprint. Private gallery photos that have never been indexed online will result in an honest `UNVERIFIED ✗` outcome.
* **Low-Resolution / Heavy Occlusion**: Candidates where face area is below 40×40 pixels or subject to severe occlusion (>60%) are discarded during candidate evaluation.
* **Testnet Latency**: Polygon Amoy block confirmation times fluctuate between 1.5s and 4.0s based on public RPC load.

---

## License

MIT License © 2026 Nikhil Kumar Panigrahi.
