// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

/**
 * @title ContentRegistry
 * @notice Anchors cryptographic SHA-256 fingerprints of discovered public content on Polygon Amoy.
 * @dev Minimal, audited, deterministic storage for ProofFace verification pipeline.
 */
contract ContentRegistry {
    struct Proof {
        bytes32 fingerprint;
        string sourceUrl;
        uint256 timestamp;
        bool exists;
    }

    // Mapping from 32-byte content fingerprint to Proof
    mapping(bytes32 => Proof) private proofs;

    event ProofRegistered(
        bytes32 indexed fingerprint,
        string sourceUrl,
        uint256 timestamp,
        address indexed registrar
    );

    /**
     * @notice Registers a new content fingerprint proof. Idempotent: does not overwrite existing proof.
     * @param fingerprint The 32-byte SHA-256 digest of canonical discovered content.
     * @param sourceUrl The public URL where content was discovered.
     */
    function registerProof(bytes32 fingerprint, string calldata sourceUrl) external returns (bool) {
        require(fingerprint != bytes32(0), "Invalid zero fingerprint");

        // Idempotent: return true if already registered
        if (proofs[fingerprint].exists) {
            return false;
        }

        proofs[fingerprint] = Proof({
            fingerprint: fingerprint,
            sourceUrl: sourceUrl,
            timestamp: block.timestamp,
            exists: true
        });

        emit ProofRegistered(fingerprint, sourceUrl, block.timestamp, msg.sender);
        return true;
    }

    /**
     * @notice Retrieves the recorded proof for a given fingerprint.
     * @param fingerprint The 32-byte SHA-256 digest.
     */
    function getProof(bytes32 fingerprint)
        external
        view
        returns (
            bytes32 _fingerprint,
            string memory _sourceUrl,
            uint256 _timestamp,
            bool _exists
        )
    {
        Proof memory p = proofs[fingerprint];
        return (p.fingerprint, p.sourceUrl, p.timestamp, p.exists);
    }
}
