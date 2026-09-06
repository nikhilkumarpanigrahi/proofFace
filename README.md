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
                     ┌─────────────────────────┐
                     │ 2. Face Detection (CNN) │
                     │ UltraFace RFB-320 ONNX  │
                     │ 4,420 Prior Anchor Box  │
                     │ + Biometric Liveness    │
                     │ (YCbCr + Texture Var)   │
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
               │ 4. Visual Search Orchestrator      │
               │                                    │
               │  Google Lens AI Vision ┐           │
               │  (High-Res CDN Fetch)  ├→ Candidates
               │  Fallback Provider ────┘           │
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

---

## Biometric Anti-Spoofing & AI/Anime Rejection (ISO/IEC 30107-3)

ProofFace enforces a rigorous, multi-stage biometric validation policy (`BiometricSecurityConfig`) directly inside the face detection pipeline:

1. **UltraFace RFB-320 CNN**: Deep convolutional neural network running 4,420 prior anchor boxes via ONNX runtime (`tract-onnx`), eliminating arbitrary heuristics.
2. **Kovac / Chai & Ngan $YC_bC_r$ Melanin Spectrum Clustering**: Converts face crops into chromatic $YC_bC_r$ space ($C_b \in [77, 127], C_r \in [133, 173]$) with Gray-World illumination normalization. Requires $\ge 32\%$ genuine human skin coverage.
3. **Spatial Laplacian Micro-Texture Variance**: Calculates discrete Laplacian spatial frequency energy ($12 \le \sigma^2 \le 6500$). Rejects flat 2D cartoon illustrations ($\sigma^2 < 10$) and solid vector fills while accommodating 4K smartphone cameras.
4. **Color Saturation PAD Filter**: Real human skin under natural lighting exhibits low-to-moderate saturation. Synthetic AI art, anime characters, and manga art use intense digital pigmentation ($S > 0.50$ across $> 60\%$ of pixels). ProofFace strictly limits hyper-saturated pixels to $\le 45\%$.
5. **Anthropometric Facial Aspect Ratios (ISO/IEC 19794-5)**: Enforces upright bounding box dimensions ($0.70 \le \text{height}/\text{width} \le 1.80$), eliminating non-human vertical strips or squashed geometries.

| Parameter | Calibrated Value | Standard / Scientific Reference | Target Defense |
| :--- | :--- | :--- | :--- |
| `min_cnn_confidence` | `0.70` | UltraFace ONNX Anchor Scoring | Weak / spurious candidate proposals |
| `min_skin_coverage_ratio` | `0.32` (32%) | Kovac et al. / Chai & Ngan ($YC_bC_r$) | Non-human animals, pets, furniture |
| `min_texture_variance` | `12.0` | Pech-Pacheco et al. (ICPR 2000) | Flat 2D cartoons, solid vector fills |
| `max_texture_variance` | `6500.0` | High-frequency optical noise bound | Synthetic ink outlines while allowing 4K HDR |
| `max_high_saturation_ratio` | `0.45` (45%) | HSV Chromaticity Analysis | AI-generated anime, manga, 3D CGI art |
| `min_physical_aspect_ratio` | `0.70` | ISO/IEC 19794-5 Cranial Biometrics | Distorted / unnatural aspect ratios |
| `max_physical_aspect_ratio` | `1.80` | ISO/IEC 19794-5 Cranial Biometrics | Elongated vertical image strips |
| `nms_iou_threshold` | `0.30` | Non-Maximum Suppression (NMS) | Duplicate candidate bounding boxes |

> **Result**: Animal photos (cats, dogs), cartoon illustrations (pandas), and AI-generated anime art are cleanly halted at Stage 2 with `Pipeline execution halted: No face detected in input image` and never anchored on-chain.

---

## Dual-Resolution Candidate Evaluation Architecture

Social media networks and image platforms present conflicting technical constraints:
* **Meta Platforms (Instagram, Threads, Facebook)**: Raw post links use anti-scraping widgets (`lookaside.instagram.com`) that return HTML login walls to non-browser requests.
* **Image Platforms (Pinterest, News, Blogs, YouTube)**: Original images are high-resolution (736×736+), whereas default thumbnails can be tiny icons (100×100) where small faces become unresolvable.

ProofFace solves this with **Automatic Dual-Resolution Fallback**:
```text
Discovered Candidate
       │
       ├─ Primary: High-Resolution Original Image (or Google CDN for Meta)
       │      │ (download failure / HTML login wall / 0 faces detected)
       │      ▼
       └─ Fallback: Google CDN Cached Thumbnail (encrypted-tbn.gstatic.com)
```

1. **Smart Prioritization**: Meta crawler links automatically select the fast, unblocked Google CDN thumbnail; other platforms select high-resolution originals.
2. **Seamless Fallback**: If an original image is blocked by hotlink protection or a thumbnail is too small to resolve facial landmarks, the pipeline immediately falls back to the alternative URL.
3. **Focused Verification Output**: Displays only the single authentic matched post on the CLI, removing irrelevant candidate noise.

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

* **Calibrated Similarity Threshold ($\tau = 0.30$)**: Tuned for real-world face orientation, lighting variations, and cross-platform compression artifacts.
* **Bounded Concurrency**: Maximum 5 concurrent candidate evaluations (`tokio::sync::Semaphore`) across up to 10 discovered candidates.
* **On-Chain Privacy & Efficiency**: Only the 32-byte cryptographic fingerprint (`bytes32`), `sourceUrl` (`string`), and timestamp are anchored on-chain. Raw images are never stored on the blockchain.
* **RFC 8785 Canonicalization**: Implemented via `serde_jcs` to ensure byte-level deterministic hashing regardless of key order, whitespace, or serializer implementation.

---

## Quickstart & Installation

### 1. Prerequisites
* **Rust & Cargo** (1.80+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### 2. Clone & Build
```bash
git clone https://github.com/nikhilkumarpanigrahi/proofFace.git
cd proofFace
cp .env.example .env
cargo build --release
```

### 3. Verify Configuration
```bash
cargo run -- health
```

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
[2/7] Detecting face... ✓ 1 face detected (confidence: 0.85)
[3/7] Generating embedding... ✓ L2-normalized 128-dim embedding generated
[4/7] Searching public web for candidates (Google Lens AI Vision)...
      ✓ 10 search candidate URLs discovered
[5/7] Verifying candidates (bounded concurrency: 5)...
      #Candidate 01 ........ similarity: 0.720 (PossibleMatch)
      #Candidate 03 ........ similarity: 0.744 (PossibleMatch)
      #Candidate 07 ........ similarity: 0.768 (PossibleMatch)
      #Candidate 09 ........ similarity: 0.929 (HighConfidence)

╔══════════════════════════════════════════════════════════╗
║             AUTHENTIC PUBLIC POST MATCHED 🌐             ║
╚══════════════════════════════════════════════════════════╝
  Platform : [Instagram]
  Title    : Iran One of my Favourite country in the world...
  URL      : https://www.instagram.com/p/CpmFTTCIjz0/
  Match    : 92.9% (✓ HighConfidence)
  Media    : https://encrypted-tbn1.gstatic.com/...

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
| `HIGH_CONFIDENCE_THRESHOLD` | Calibrated cosine similarity threshold ($\tau$) | `0.30` |
| `POSSIBLE_MATCH_THRESHOLD` | Ambiguous similarity threshold | `0.15` |
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
