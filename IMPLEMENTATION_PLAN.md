# ProofFace — Implementation Plan

**Version:** 1.0  
**Status:** Ready for implementation  
**Primary language:** Rust  
**Approach:** Vertical-slice first, resilience second, polish third.

---

# 1. Engineering Strategy

The project must be built in this order:

```text
Working core
    ↓
Real external integrations
    ↓
Blockchain proof
    ↓
End-to-end verification
    ↓
Failure handling
    ↓
Performance
    ↓
Demo polish
```

Do not build every advanced system-design pattern before the happy path works.

### Golden rule

> Build the smallest real system that satisfies the requirement, then harden the actual failure points.

---

# 2. Phase 0 — Repository and Tooling

## Objective

Create a clean Rust repository and development environment.

### Tasks

- Create GitHub repository.
- Initialize Rust project.
- Add `.gitignore`.
- Add `.env.example`.
- Add README placeholder.
- Add PRD.
- Add implementation plan.
- Configure formatting.
- Configure linting.
- Add test command.
- Verify release build.

### Commands

```bash
cargo new proofface
cd proofface

git init
cargo build
cargo test
cargo fmt --check
```

### Deliverable

```text
cargo build
cargo test
```

must succeed.

---

# 3. Phase 1 — Project Skeleton

Create:

```text
src/
├── main.rs
├── config.rs
├── error.rs
├── models.rs
├── pipeline.rs
├── face/
├── search/
├── content/
├── crypto/
├── blockchain/
└── resilience/
```

### Dependencies to evaluate

- `tokio`
- `reqwest`
- `serde`
- `serde_json`
- `dotenvy`
- `thiserror`
- `anyhow` where appropriate at application boundaries
- `tracing`
- `tracing-subscriber`
- `clap`
- `sha2`
- `chrono`

For blockchain:

- Prefer maintained `alloy` ecosystem unless compatibility requires another choice.

For image/model execution:

- Evaluate current Rust-compatible `image`, OpenCV, and ONNX Runtime bindings.
- Choose based on actual model compatibility and stability, not popularity alone.

### Deliverable

CLI starts and prints a version/help message.

---

# 4. Phase 2 — Configuration and Logging

## Implement

Environment configuration:

```text
SEARCH_API_KEY
SEARCH_PROVIDER
RPC_PRIMARY
RPC_SECONDARY
WALLET_PRIVATE_KEY
CONTRACT_ADDRESS
CHAIN_ID
MAX_CONCURRENT_CANDIDATES
SEARCH_TIMEOUT_MS
MAX_RETRIES
MATCH_THRESHOLD
```

Never hardcode secrets.

### Logging

Use structured logs:

```text
INFO face.detected count=1
INFO search.provider provider=a
WARN search.timeout provider=a
INFO search.fallback provider=b
INFO match.found similarity=0.93
INFO blockchain.confirmed tx=0x...
```

### Acceptance

- `.env` loads locally.
- `.env.example` contains placeholders only.
- Secrets never appear in logs.

---

# 5. Phase 3 — Real Face Engine

## Goal

Prove Rust can process the real input image.

### Steps

1. Load image.
2. Validate image.
3. Detect faces.
4. Select target face.
5. Generate embedding.
6. Normalize embedding if required.
7. Expose a model-independent interface.

Conceptual API:

```rust
trait FaceEngine {
    async fn detect(&self, image: &[u8]) -> Result<Vec<DetectedFace>>;
    async fn embed(&self, face: &DetectedFace) -> Result<FaceEmbedding>;
}
```

### Test cases

```text
valid face image       → success
no face                → NO_FACE_DETECTED
multiple faces         → explicit handling
invalid image          → INVALID_IMAGE
```

### Exit criteria

A real image produces a real embedding.

Do not proceed until this works reliably.

---

# 6. Phase 4 — Similarity Engine

Implement:

```text
embedding A
embedding B
      ↓
cosine similarity
```

Test:

- Same face → high similarity.
- Different faces → lower similarity.
- Dimension mismatch → typed error.
- Empty vectors → typed error.

Keep the similarity engine independent of the model implementation.

---

# 7. Phase 5 — Search Provider Research Spike

This is the largest technical risk.

Before writing the complete search architecture, verify that the chosen service can:

1. Accept the type of search input we can legally provide.
2. Return genuine web/public content.
3. Expose candidate URLs.
4. Provide usable media/content for downstream verification.
5. Work within the hackathon budget.
6. Have acceptable rate limits.

### Decision rule

Do not lock the project to a provider until a real API call has returned usable candidates.

### Important

The demo path must use the real provider.

No hardcoded URL.

---

# 8. Phase 6 — Search Provider Abstraction

Create:

```rust
trait SearchProvider {
    async fn search(
        &self,
        request: SearchRequest,
    ) -> Result<Vec<SearchResult>, ProviderError>;
}
```

Implement the primary provider first.

Provider-specific parsing stays inside its module.

The pipeline must know only about the abstraction.

---

# 9. Phase 7 — Search Resilience

Add:

## Timeout

Every request gets a deadline.

## Retry

Limited attempts.

Example:

```text
attempt 1
↓
backoff
attempt 2
↓
backoff
attempt 3
↓
fallback
```

## Fallback

```text
Provider A → failure
Provider B → success
```

## Circuit breaker

Implement only after the above works.

### Exit criteria

A deliberate provider failure does not terminate the application if another configured provider is healthy.

---

# 10. Phase 8 — Candidate Pipeline

Search results:

```text
SearchResult[]
```

become:

```text
Candidate[]
```

Pipeline:

```text
URL
 ↓
validate
 ↓
fetch
 ↓
validate content
 ↓
extract image
 ↓
detect face
 ↓
embedding
 ↓
similarity
```

Individual candidate failures should be isolated.

Example:

```text
Candidate 1 → invalid → skip
Candidate 2 → timeout → skip
Candidate 3 → valid → process
```

---

# 11. Phase 9 — Bounded Concurrency

Use Tokio tasks with a semaphore or equivalent bounded mechanism.

Example:

```text
MAX_CONCURRENT_CANDIDATES=5
```

Do not spawn unbounded tasks.

### Why

Protect:

- memory
- CPU
- remote services
- rate limits

### Benchmark

Compare:

```text
sequential
```

vs

```text
bounded concurrency
```

on a controlled candidate set.

---

# 12. Phase 10 — Candidate Ranking

For every usable candidate:

```text
face_similarity
source relevance
content validity
```

produce a rank.

At minimum:

```text
similarity score
```

must be available.

Return:

```text
HIGH_CONFIDENCE_MATCH
POSSIBLE_MATCH
NO_MATCH
```

Do not claim certainty beyond what the model/threshold supports.

---

# 13. Phase 11 — Early Termination

If a candidate crosses the high-confidence threshold and all required validation checks pass:

```text
stop unnecessary candidate work
```

This reduces:

- CPU work
- downloads
- API usage
- latency

Do not early-stop merely because the search provider ranked a result first.

---

# 14. Phase 12 — Candidate Deduplication

Implement:

1. URL normalization.
2. Duplicate URL removal.
3. Media URL deduplication.
4. Optional content hash.
5. Optional perceptual image hash if it provides real value.

Do not add a database just for deduplication.

An in-memory set is sufficient for the hackathon pipeline.

---

# 15. Phase 13 — Content Extraction

Create:

```rust
DiscoveredContent
```

with:

- source URL
- media URL
- text/caption if available
- deterministic metadata
- retrieval timestamp

Keep extraction separate from hashing.

---

# 16. Phase 14 — Canonicalization

This is a critical integrity component.

### Process

```text
DiscoveredContent
 ↓
remove volatile fields
 ↓
normalize text
 ↓
normalize representation
 ↓
deterministic serialization
```

Then test:

```text
same logical content
→ same canonical bytes
→ same SHA-256
```

---

# 17. Phase 15 — SHA-256

Use a standard audited crypto crate.

Conceptually:

```rust
Sha256::digest(canonical_bytes)
```

Output:

```text
32 bytes
```

Represent as:

```text
0x...
```

or another documented deterministic format.

### Tests

```text
same bytes → same hash
one byte changed → different hash
field changed → different hash
```

---

# 18. Phase 16 — Smart Contract

Create:

```text
contracts/ContentRegistry.sol
```

Minimal contract responsibilities:

```text
registerProof
getProof
```

Store:

```text
fingerprint
source
timestamp
```

Avoid storing large data.

---

# 19. Phase 17 — Deploy to Polygon Amoy

Steps:

1. Create test wallet.
2. Obtain testnet tokens from an appropriate faucet.
3. Deploy contract.
4. Record contract address.
5. Verify deployment.
6. Put address in `.env`.

Never commit private key.

---

# 20. Phase 18 — Rust Blockchain Adapter

Create an abstraction:

```rust
trait BlockchainRegistry {
    async fn register_proof(
        &self,
        proof: ContentProof,
    ) -> Result<TransactionReceipt, BlockchainError>;

    async fn get_proof(
        &self,
        fingerprint: Fingerprint,
    ) -> Result<Option<OnChainProof>, BlockchainError>;
}
```

Implement Polygon Amoy adapter.

---

# 21. Phase 19 — Blockchain RPC Resilience

Configure:

```text
RPC_PRIMARY
RPC_SECONDARY
```

Failure:

```text
A timeout
 ↓
B request
 ↓
success
```

Add:

- timeout
- limited retry
- backoff
- failover

Optional later:

- health scoring
- circuit breaker
- quorum reads

Do not implement advanced routing before the basic failover works.

---

# 22. Phase 20 — Idempotency and Transaction Uncertainty

Important distributed-systems case:

```text
submit transaction
↓
network timeout
↓
client doesn't know whether transaction landed
```

Do not immediately submit another transaction.

Instead:

1. Check whether proof already exists.
2. Check transaction state if transaction hash is known.
3. Only submit a new proof if necessary.

Use the fingerprint as the logical proof identifier.

---

# 23. Phase 21 — Read-After-Write Verification

After registration:

```text
local fingerprint
       ↓
submit
       ↓
confirmation
       ↓
read on-chain record
       ↓
compare
```

Output:

```text
MATCH → VERIFIED
MISMATCH → VERIFICATION_FAILED
NO RECORD → UNVERIFIED
```

---

# 24. Phase 22 — Tamper Demonstration

Create a reproducible local demonstration.

### Original

```text
content
 ↓
hash A
 ↓
blockchain
 ↓
verify
 ↓
VERIFIED
```

### Modified

```text
modified content
 ↓
hash B
 ↓
compare against A
 ↓
TAMPERED
```

This must be real cryptographic verification.

---

# 25. Phase 23 — Pipeline State Machine

Represent major stages explicitly.

```text
INITIALIZED
 ↓
IMAGE_VALIDATED
 ↓
FACE_DETECTED
 ↓
FACE_ENCODED
 ↓
SEARCHING
 ↓
CANDIDATES_FOUND
 ↓
MATCHING
 ↓
MATCH_FOUND
 ↓
CONTENT_FETCHED
 ↓
FINGERPRINT_CREATED
 ↓
BLOCKCHAIN_SUBMITTED
 ↓
BLOCKCHAIN_CONFIRMED
 ↓
VERIFYING
 ↓
VERIFIED
```

Failure states:

```text
INVALID_IMAGE
NO_FACE
SEARCH_FAILED
NO_MATCH
CONTENT_UNAVAILABLE
BLOCKCHAIN_FAILED
VERIFICATION_FAILED
UNVERIFIED
```

---

# 26. Phase 24 — Circuit Breaker

Implement only after real provider failure testing.

For each provider:

```text
CLOSED
 ↓ repeated failures
OPEN
 ↓ cooldown
HALF_OPEN
 ↓ health success
CLOSED
```

During `OPEN`, route to fallback.

---

# 27. Phase 25 — Bulkhead and Resource Limits

Keep independent limits for:

- candidate downloads
- face inference tasks
- blockchain operations

Use semaphores/queues where necessary.

Do not build a distributed queue.

---

# 28. Phase 26 — Caching

Optional.

Potential cache:

```text
candidate media hash
       ↓
existing embedding
```

Only add if repeated candidate processing is observed.

For the initial version, in-memory caching is enough.

---

# 29. Phase 27 — Observability

Add:

- stage timing
- provider name
- retry count
- fallback event
- candidate count
- match score
- blockchain transaction hash
- final status

Example:

```text
INFO stage=face duration_ms=184
INFO stage=search provider=a duration_ms=503
WARN stage=search provider=a event=timeout
INFO stage=search provider=b duration_ms=820
INFO stage=matching candidates=14
INFO stage=match similarity=0.934
INFO stage=hash algorithm=sha256
INFO stage=blockchain tx=0x...
INFO stage=verification status=verified
```

---

# 30. Phase 28 — Security Review

Before demo:

### Secrets

```text
.env ignored
private key not logged
API key not logged
```

### Network

```text
timeouts
redirect limits
download limits
MIME validation
```

### Input

```text
image size limits
dimension limits
malformed image handling
```

### Privacy

Use appropriate public/consented demo data.

---

# 31. Phase 29 — Test Matrix

## Face

| Case | Expected |
|---|---|
| Valid face | Success |
| No face | `NO_FACE_DETECTED` |
| Multiple faces | Explicit handling |
| Corrupt image | `INVALID_IMAGE` |

## Search

| Case | Expected |
|---|---|
| Provider succeeds | Results |
| Provider timeout | Retry |
| Provider unavailable | Fallback |
| No candidates | `NO_MATCH` |

## Candidate

| Case | Expected |
|---|---|
| Valid candidate | Process |
| Invalid image | Skip |
| No face | Skip |
| Weak similarity | Reject |
| Strong similarity | Candidate match |

## Blockchain

| Case | Expected |
|---|---|
| RPC primary works | Use primary |
| RPC primary fails | Secondary |
| Transaction confirmed | Continue |
| Existing proof | Avoid duplicate |
| Hash mismatch | `TAMPERED` |

---

# 32. Phase 30 — Performance Work

Only optimize after correctness.

### Benchmark

```text
10 candidates
50 candidates
100 candidates
```

Compare:

```text
sequential
vs
bounded concurrency
```

Measure:

```text
wall-clock time
CPU time where practical
memory where practical
```

Also measure:

```text
face inference
hashing
network search
blockchain confirmation
```

Keep network latency separate from compute benchmarks.

---

# 33. Phase 31 — CLI Polish

Make the output easy to understand.

Use:

```text
✓ success
⚠ fallback
✗ failure
```

but preserve machine-readable internal status.

Final statuses:

```text
VERIFIED
TAMPERED
UNVERIFIED
FAILED
```

---

# 34. Phase 32 — README

README sections:

```text
# ProofFace

## What it does
## Why it exists
## Architecture
## System design
## Features
## Tech stack
## Requirements
## Installation
## Environment setup
## Search provider setup
## Blockchain setup
## Smart contract deployment
## Running the pipeline
## Example output
## Failure handling
## Tamper verification
## Benchmarks
## Security
## Privacy
## Limitations
## Demo
```

Include architecture diagram.

---

# 35. Phase 33 — Demo Recording

Record one clean run.

### Run 1

```text
cargo run --release -- verify ./samples/input.jpg
```

Show:

```text
face detected
↓
real search
↓
candidate
↓
match score
↓
hash
↓
blockchain TX
↓
on-chain verification
↓
VERIFIED
```

### Run 2

Show tampered content:

```text
modified
↓
different hash
↓
TAMPERED
```

No editing required.

---

# 36. Phase 34 — Final Submission Review

Before submission verify:

```text
[ ] GitHub repository public/accessible as required
[ ] README works from a clean environment
[ ] No secrets committed
[ ] Contract address documented
[ ] Demo uses real data
[ ] Search is not hardcoded
[ ] Blockchain TX is real
[ ] Verification is reproducible
[ ] Recording link works
[ ] Submission form fields ready
```

Because the challenge says **no resubmissions**, do a final dry-run from a clean terminal before submitting.

---

# 37. Recommended Git Commit Plan

Use meaningful commits:

```text
chore: initialize rust project
feat: add configuration and logging
feat: implement face detection
feat: implement face embeddings
feat: add similarity engine
feat: add search provider abstraction
feat: integrate real search provider
feat: add candidate verification
feat: add bounded concurrency
feat: add search fallback
feat: add content canonicalization
feat: add sha256 fingerprinting
feat: add content registry contract
feat: integrate polygon amoy
feat: add blockchain verification
feat: add rpc failover
feat: add tamper detection
test: add pipeline failure tests
perf: benchmark candidate processing
docs: add architecture and setup
docs: add demo instructions
```

---

# 38. Agent Execution Rules

Give these instructions to the coding agent:

```text
You are the lead Rust engineer implementing ProofFace.

Follow the PRD and implementation plan in order.

Do not implement the entire project in one pass.

For every phase:

1. Inspect the existing repository.
2. Make the smallest change necessary.
3. Compile with cargo check/build.
4. Run relevant tests.
5. Fix errors.
6. Update documentation if behavior changed.
7. Only then continue.

Do not use hardcoded external search results.

Do not fabricate blockchain transactions.

Do not fake verification.

Mocks are allowed only inside isolated automated tests.

The real demo path must use real external services and a real Polygon Amoy transaction.

Keep providers behind traits/interfaces.

Prefer a modular monolith over microservices.

Do not add infrastructure that is not required.

Do not introduce Python unless a specific Rust implementation is genuinely blocked after reasonable investigation.

Do not claim identity certainty beyond the model's evidence.

Never return VERIFIED unless the content fingerprint has actually been compared against an on-chain proof.

If evidence is insufficient, return UNVERIFIED.

Prioritize correctness and reliability over cosmetic features.

Do not add advanced system-design patterns merely for appearance.

Add complexity only when it directly improves a requirement or solves a demonstrated failure mode.

Before finalizing, run the complete test suite and a real end-to-end demo.
```

---

# 39. Priority Levels

## P0 — Must work

```text
Rust application
Face detection
Face embedding
Real search
Candidate matching
SHA-256
Smart contract
Polygon Amoy
Blockchain verification
Tamper detection
README
Demo
```

## P1 — Strong differentiators

```text
Search fallback
RPC fallback
Timeout
Retry
Bounded concurrency
Deduplication
Early termination
Structured errors
Structured logs
```

## P2 — Only if time remains

```text
Circuit breaker
Adaptive provider health
Hedged requests
Caching
Quorum reads
Bulkhead isolation
Advanced benchmarks
```

Never sacrifice P0 for P2.

---

# 40. Final Architecture Target

```text
                    ┌───────────────┐
                    │  INPUT IMAGE  │
                    └───────┬───────┘
                            ↓
                    IMAGE VALIDATOR
                            ↓
                     FACE ENGINE
                     /          \
                  Detect       Embed
                     \          /
                       SEARCH
                         │
              ┌──────────┴──────────┐
              ↓                     ↓
        Provider A              Provider B
              \                     /
               └──────────┬─────────┘
                          ↓
                   CANDIDATE POOL
                          ↓
                  DEDUPLICATION
                          ↓
                BOUNDED CONCURRENCY
                          ↓
                 FACE MATCHING
                          ↓
                    BEST MATCH
                          ↓
                 CONTENT FETCH
                          ↓
                 CANONICALIZATION
                          ↓
                      SHA-256
                          ↓
              BLOCKCHAIN REGISTRY
                    /          \
                   ↓            ↓
                RPC A         RPC B
                   \            /
                    POLYGON AMOY
                          ↓
                 READ-VERIFY
                    /       \
                   ↓         ↓
             HASH MATCH   HASH DIFFER
                   ↓         ↓
              VERIFIED    TAMPERED
```

This architecture should remain a **single Rust application with clear internal modules**, not a distributed microservice deployment.
