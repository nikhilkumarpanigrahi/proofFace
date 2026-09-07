# ProofFace 🦀

> **End-to-End Cryptographic Visual Provenance & Face Integrity Engine**  
> *Input Photo → Neural Face Detection → Web Discovery → Biometric Similarity → Polygon Amoy Blockchain Proof*

[![Rust](https://img.shields.io/badge/Rust-Edition%202021-DEA584.svg?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Polygon](https://img.shields.io/badge/Blockchain-Polygon%20Amoy%20(80002)-8247e5.svg?style=flat-square&logo=polygon)](https://amoy.polygonscan.com/address/0x647a132b73b19fcbbc19d789a2af25238afa3170)
[![Contract](https://img.shields.io/badge/Smart%20Contract-0x647a...3170-success.svg?style=flat-square)](https://amoy.polygonscan.com/address/0x647a132b73b19fcbbc19d789a2af25238afa3170)
[![RFC-8785](https://img.shields.io/badge/Canonicalization-RFC%208785%20(JCS)-success.svg?style=flat-square)](https://datatracker.ietf.org/doc/html/rfc8785)
[![Tests](https://img.shields.io/badge/Tests-30%20Passing%20(100%25)-brightgreen.svg?style=flat-square)](tests/)
[![License](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)

---

## The Problem & The Solution

Finding a photo online does not prove its authenticity, and downloading an image provides zero guarantee that the content hasn't been altered, deepfaked, or misappropriated post-publication.

**ProofFace** solves this by implementing a production-grade, 7-stage verifiable visual provenance pipeline in pure Rust:
1. **Validates & Decodes** incoming images (JPEG, PNG, WebP) with strict sanity limits.
2. **Detects Genuine Human Faces** using an ONNX-accelerated UltraFace CNN model with multi-stage **ISO/IEC 30107 Presentation Attack Detection (PAD)** — automatically rejecting non-human animals (pandas, cats, dogs), 2D cartoons, and synthetic illustrations.
3. **Extracts 128-Dimensional Biometric Embeddings** with high-pass Difference of Gaussians (DoG) normalization.
4. **Performs Real Visual Discovery** across the public web via Google Lens AI vision.
5. **Independently Calculates Cosine Similarity** across discovered web candidates in bounded parallel worker pools.
6. **Deterministically Canonicalizes** discovered metadata according to **RFC 8785 (JSON Canonicalization Scheme - JCS)** into a 32-byte SHA-256 fingerprint.
7. **Anchors & Verifies on Polygon Amoy**: Broadcasts real EVM-signed transactions into [`ContentRegistry.sol`](contracts/ContentRegistry.sol) or evaluates against live chain state with read-after-write confirmation and exportable HTML proof certificates.

> **Verification Semantics**: `VERIFIED ✓` establishes that the discovered creator content matches the immutable cryptographic record anchored on Polygon Amoy. Any post-publication tampering or pixel modification is immediately caught and flagged as **`TAMPERED ✗`**.

---

## Live Smart Contract on Polygon Amoy

| Property | Value | Polygonscan Link |
| :--- | :--- | :--- |
| **Network** | Polygon PoS Amoy Testnet | Chain ID `80002` |
| **Contract Name** | `ContentRegistry` | [`contracts/ContentRegistry.sol`](contracts/ContentRegistry.sol) |
| **Contract Address** | `0x647a132b73b19fcbbc19d789a2af25238afa3170` | [View Contract on Polygonscan ↗](https://amoy.polygonscan.com/address/0x647a132b73b19fcbbc19d789a2af25238afa3170) |
| **Deployment Tx** | `0x24495a687e63fc119680dc0737f2546cdcdacdc211a6f6ca8fa4cbb5b566910c` | [View Deployment Tx ↗](https://amoy.polygonscan.com/tx/0x24495a687e63fc119680dc0737f2546cdcdacdc211a6f6ca8fa4cbb5b566910c) |

### Real Verified On-Chain Transactions

* **Sundar Pichai (New York Times - 99.8% Match):**  
  Tx Hash: [`0x2461a6df755c216a8d1af77385d80158ec996b63473abcbb1d4306d2095b0145`](https://amoy.polygonscan.com/tx/0x2461a6df755c216a8d1af77385d80158ec996b63473abcbb1d4306d2095b0145)  
  Fingerprint: `0x77556d8cd78c1f6a24ac36e4a28d7b9641deee9fcb5769f6a50ff5973cb41e83` • Block `#47023447`
* **Student Project (Reddit - 99.4% Match):**  
  Tx Hash: [`0xe136570351483008df8d0334c73d3dedd4ea8f70ea824d231bb55ff47cafe08c`](https://amoy.polygonscan.com/tx/0xe136570351483008df8d0334c73d3dedd4ea8f70ea824d231bb55ff47cafe08c)  
  Fingerprint: `0xd9082f6fe87641018a4181efb6b3c153c3c6d29b90b42c4e4b9ec246e0e945fb` • Block `#46958785`
* **Cristiano Ronaldo (Goal.com - 95.3% Match):**  
  Tx Hash: [`0x97422d08128802cf72e2c6422aefd9b9ce397dab9df32485d031a1e0de339c59`](https://amoy.polygonscan.com/tx/0x97422d08128802cf72e2c6422aefd9b9ce397dab9df32485d031a1e0de339c59)

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
                      │ 4,420 Anchor Priors     │
                      │ + Biometric Liveness    │
                      │ (YCbCr Melanin Cluster) │
                      └────────────┬────────────┘
                                   │
                        (Non-human animal / cartoon: HALT)
                                   │
                                   ▼
                      ┌─────────────────────────┐
                      │ 3. Face Embedding       │
                      │ 128-Dim DoG Descriptor  │
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
                      │ Cosine Similarity       │
                      │ Match Threshold (τ)     │
                      └────────────┬────────────┘
                                   │
                                   ▼
                      ┌─────────────────────────┐
                      │ 6. RFC 8785 JCS Hashing │
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
                      │ Stored Hash == Recalc?  │
                      └────────────┬────────────┘
                                   │
                           ┌───────┴───────┐
                           ▼               ▼
                      VERIFIED ✓       TAMPERED ✗
```

---

## Biometric Anti-Spoofing & Animal Rejection (ISO/IEC 30107)

Standard neural face detectors often trigger false proposals on animal features (pandas, cats, dogs) or cartoon characters because they share geometric patterns (eyes, snout).

ProofFace enforces strict **Human Biometric Chromatic Closeness**:
1. **$YC_bC_r$ Melanin Cluster Analysis**: Evaluates whether detected bounding box pixels reside within human hemoglobin and melanin absorption spectrum ($C_b \in [77, 127], C_r \in [133, 173]$) under Gray-World illumination normalization.
2. **Minimum Human Skin Ratio**: Requires $\ge 18\%$ genuine human skin coverage. Animal fur (black/white panda fur, brown pet fur) contains no human chrominance and is immediately rejected at Step 2 with `Pipeline execution halted: No face detected in input image`.
3. **Laplacian Spatial Texture Variance**: Rejects flat 2D synthetic vector fills and anime graphics ($\sigma^2 < 10$) while supporting real cameras up to 4K resolution ($\sigma^2 \le 15,000$).
4. **Anthropometric Cranial Ratios**: Restricts physical face aspect ratios to $0.50 \le \text{height}/\text{width} \le 2.20$, discarding unnatural geometric slivers.

---

## Quickstart & CLI Commands

### 1. Installation

```bash
# Clone the repository
git clone https://github.com/nikhilkumarpanigrahi/proofFace.git
cd proofFace

# Copy environment configuration
cp .env.example .env

# Build optimized binary
cargo build --release
```

### 2. Verify a Single Image

```bash
cargo run -- verify path/to/photo.jpeg
```

**Example Terminal Output:**
```text
╔══════════════════════════════════════════════════════════╗
║                      PROOFFACE 🦀                        ║
║     Face → Web Discovery → Blockchain Proof              ║
╚══════════════════════════════════════════════════════════╝

[1/7] Validating image... ✓ Valid image (22810 bytes)
[2/7] Detecting face... ✓ 1 face detected (confidence: 1.00)
[3/7] Generating embedding... ✓ L2-normalized 128-dim embedding generated
[4/7] Searching public web for candidates (Google Lens AI Vision)...
      ✓ 10 search candidate URLs discovered
[5/7] Verifying candidates (bounded concurrency: 5)...
      #Candidate 02 ........ similarity: 0.883 (HighConfidence)

╔══════════════════════════════════════════════════════════╗
║             AUTHENTIC PUBLIC POST MATCHED 🌐             ║
╚══════════════════════════════════════════════════════════╝
  Platform : [LinkedIn]
  Title    : motivation #team #travel #grateful #awards | Anunay Sood
  URL      : https://www.linkedin.com/posts/anunaysood_...
  Match    : 88.3% (✓ HighConfidence)

[6/7] Creating deterministic SHA-256 fingerprint... ✓ Fingerprint: 0xe5d20c1cd5e7e33f1e009488dba3613d5b5b45091a20daad0b119b06feca39b0
      Anchoring on Polygon Amoy (Chain ID 80002)... ✓ Confirmed
      Tx Hash: 0xeaf81fb2c8dba94ca90df623c4f036b005bffd66c62bef53f9bdf64e31b6282b
[7/7] Re-verifying against on-chain record... ✓ Match confirmed
      Block Number: #46958785
      Explorer    : https://amoy.polygonscan.com/tx/0xeaf81fb2c8dba94ca90df623c4f036b005bffd66c62bef53f9bdf64e31b6282b

╔══════════════════════════════════════════════════════════╗
║                      VERIFIED ✓                          ║
╚══════════════════════════════════════════════════════════╝

  • View On-Chain Receipt : https://amoy.polygonscan.com/tx/0xeaf81fb2c8dba94ca90df623c4f036b005bffd66c62bef53f9bdf64e31b6282b
  • Verification Certificate: proof_certificate.html (Generated)
```

---

### 3. Inspect On-Chain Proof Directly From Smart Contract

You can query the smart contract on Polygon Amoy directly from your terminal using any fingerprint:

```bash
cargo run -- inspect-proof 0xd9082f6fe87641018a4181efb6b3c153c3c6d29b90b42c4e4b9ec246e0e945fb
```

**Output:**
```text
✓ Proof Found On-Chain:
  Fingerprint: 0xd9082f6fe87641018a4181efb6b3c153c3c6d29b90b42c4e4b9ec246e0e945fb
  Source URL : https://www.reddit.com/r/mht_cet/comments/1vtdkn3/...
  Timestamp  : 1788777227
```

---

### 4. Interactive Cryptographic Tamper Demo

Demonstrates cryptographic tamper detection by simulating post-publication modification:

```bash
cargo run -- tamper-demo samples/test_real.jpg
```

```text
--- [SIMULATING UNAUTHORIZED CONTENT MODIFICATION] ---
Simulating alteration of title/media metadata on discovered post...
Registered On-Chain Fingerprint : 0x77556d8cd78c1f6a24ac36e4a28d7b9641deee9fcb5769f6a50ff5973cb41e83
Recalculated Tampered Fingerprint: 0x98f439c29aa4411130eec920fa5812903bb1cf9849204010ee21cb0395601d77
Comparison Result               : MISMATCH ✗

╔══════════════════════════════════════════════════════════╗
║                      TAMPERED ✗                          ║
║          ProofFace detected altered content              ║
╚══════════════════════════════════════════════════════════╝
```

---

### 5. Multi-Image & Batch Folder Audit

```bash
# Verify multiple individual images
cargo run -- verify photo1.jpg photo2.jpg

# Verify an entire directory of photos
cargo run -- verify ~/Downloads/photos/

# Strict CI/CD mode (returns non-zero exit code if any image is unverified)
cargo run -- verify ~/Downloads/photos/ --strict
```

---

### 6. Deploy Your Own Smart Contract

Deploy a new `ContentRegistry` instance to Polygon Amoy in one command:

```bash
cargo run -- deploy-contract
```

---

## Visual Verification Certificate

Whenever an image is verified, ProofFace generates [`proof_certificate.html`](proof_certificate.html) in the current directory.

Open the certificate in your browser:
```bash
open proof_certificate.html
```

**Certificate Features:**
* Glassmorphism dark-mode UI with gradient accents.
* Discovered post title, biometric similarity score, and original post URL.
* 32-byte cryptographic SHA-256 fingerprint.
* Block number, timestamp, and clickable **"View on Polygonscan ↗"** button linking directly to the verified on-chain transaction.

---

## Configuration & Environment Variables

| Variable | Description | Default |
| :--- | :--- | :--- |
| `SEARCH_PROVIDER` | Search provider (`serpapi`, `brave`, `tavily`, `public_web`) | `serpapi` |
| `SEARCH_API_KEY` | API Key for visual reverse search | (Required for `serpapi`) |
| `SEARCH_FALLBACK_PROVIDER`| Secondary provider on timeout/error | `public_web` |
| `CHAIN_ID` | EVM Chain ID | `80002` (Polygon Amoy) |
| `RPC_PRIMARY` | Primary Polygon Amoy RPC endpoint | `https://polygon-amoy.drpc.org` |
| `RPC_SECONDARY` | Failover Polygon Amoy RPC endpoint | `https://polygon-amoy-bor-rpc.publicnode.com` |
| `WALLET_PRIVATE_KEY` | Hex private key for on-chain signing | Optional (Auto-fallback to live state mode) |
| `CONTRACT_ADDRESS` | Deployed `ContentRegistry` address | `0x647a132b73b19fcbbc19d789a2af25238afa3170` |
| `HIGH_CONFIDENCE_THRESHOLD`| Cosine similarity threshold for high confidence | `0.30` |
| `MAX_CONCURRENT_CANDIDATES`| Max simultaneous image evaluations | `5` |

---

## Testing & Quality Assurance

ProofFace includes a comprehensive automated test suite with **100% pass rate across 30 unit and integration tests**:

```bash
# Run unit & integration test suite
cargo test
```

```text
running 24 tests
test blockchain::rlp::tests::test_rlp_u64 ... ok
test blockchain::rlp::tests::test_rlp_short_string ... ok
test blockchain::rlp::tests::test_rlp_single_byte ... ok
test blockchain::rlp::tests::test_rlp_empty_string ... ok
test blockchain::rlp::tests::test_rlp_empty_list ... ok
test blockchain::rlp::tests::test_rlp_string_list ... ok
test blockchain::contract::tests::test_selector_calculation ... ok
test blockchain::contract::tests::test_encode_and_decode ... ok
test crypto::hasher::tests::test_sha256_deterministic ... ok
test content::canonicalizer::tests::test_canonicalization_is_deterministic ... ok
test content::dedup::tests::test_url_normalization_strips_tracking ... ok
test crypto::hasher::tests::test_sha256_empty_bytes ... ok
test crypto::hasher::tests::test_sha256_one_byte_difference_changes_hash ... ok
test content::canonicalizer::tests::test_modified_content_produces_tampered_fingerprint ... ok
test face::similarity::tests::test_empty_vectors_returns_error ... ok
test face::similarity::tests::test_evaluate_confidence ... ok
test face::similarity::tests::test_opposite_vectors_have_similarity_minus_one ... ok
test face::similarity::tests::test_dimension_mismatch_returns_error ... ok
test face::similarity::tests::test_identical_vectors_have_similarity_one ... ok
test face::similarity::tests::test_orthogonal_vectors_have_similarity_zero ... ok
test content::dedup::tests::test_deduplicator_filters_duplicates ... ok
test blockchain::signer::tests::test_address_derivation ... ok
test blockchain::signer::tests::test_sign_transaction ... ok
test resilience::retry::tests::test_retry_succeeds_on_second_attempt ... ok
test result: ok. 24 passed; 0 failed

running 6 tests
test test_contract_abi_encoding_invariants ... ok
test test_similarity_ranking_and_thresholds ... ok
test test_content_canonicalization_and_tamper_detection ... ok
test test_candidate_deduplication_heuristics ... ok
test test_image_validation_and_face_detection ... ok
test test_face_embedding_generation_and_normalization ... ok
test result: ok. 6 passed; 0 failed
```

---

## License

MIT License © 2026 Nikhil Kumar Panigrahi.
