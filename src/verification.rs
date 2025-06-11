use sp_std::prelude::*;
use sp_runtime::traits::Hash;
use codec::{Decode, Encode};
use frostgate_circuits::sp1::{Sp1Backend, Sp1Config};
use frostgate_zkip::{ZkBackend, ZkError};

/// Verification error types
#[derive(Debug, Encode, Decode, PartialEq, Eq)]
pub enum VerificationError {
    /// Invalid proof format
    InvalidProofFormat,
    /// Proof verification failed
    VerificationFailed,
    /// Invalid input format
    InvalidInput,
    /// System error
    SystemError,
    /// Backend error
    BackendError(Vec<u8>),
}

impl From<ZkError> for VerificationError {
    fn from(error: ZkError) -> Self {
        match error {
            ZkError::Program(msg) => VerificationError::InvalidProofFormat,
            ZkError::VerificationFailed(msg) => VerificationError::VerificationFailed,
            ZkError::Input(msg) => VerificationError::InvalidInput,
            _ => VerificationError::SystemError,
        }
    }
}

/// Result type for verification operations
pub type VerificationResult = Result<(), VerificationError>;

/// Proof verification context
#[derive(Clone)]
pub struct VerificationContext {
    /// Program bytes
    pub program: Vec<u8>,
    /// Program hash
    pub program_hash: [u8; 32],
    /// Backend instance
    pub backend: Sp1Backend,
}

impl VerificationContext {
    /// Create a new verification context
    pub fn new(program: Vec<u8>, program_hash: [u8; 32]) -> Self {
        let config = Sp1Config {
            max_concurrent: Some(2), // Limited concurrency for on-chain verification
            cache_size: 10,         // Small cache for on-chain use
            use_gpu: false,         // No GPU for on-chain verification
        };
        
        Self {
            program,
            program_hash,
            backend: Sp1Backend::with_config(config),
        }
    }
}

/// Proof verification parameters
pub struct VerificationParams<'a> {
    /// Proof bytes
    pub proof: &'a [u8],
    /// Public input
    pub input: &'a [u8],
    /// Source chain ID
    pub from_chain: u64,
    /// Destination chain ID
    pub to_chain: u64,
    /// Message nonce
    pub nonce: u64,
    /// Message timestamp
    pub timestamp: u64,
}

/// Verify a proof using the configured backend
pub async fn verify_proof(
    context: &VerificationContext,
    params: &VerificationParams<'_>,
) -> VerificationResult {
    // Verify the proof
    context.backend.verify(&context.program, params.proof, None)
        .await
        .map_err(|e| e.into())
        .and_then(|valid| {
            if valid {
                Ok(())
            } else {
                Err(VerificationError::VerificationFailed)
    }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sp_core::H256;

    #[tokio::test]
    async fn test_proof_verification() {
        // Create a dummy program and hash
        let program = vec![1, 2, 3, 4];
        let program_hash = H256::from_slice(&[0; 32]).into();

        // Create verification context
        let context = VerificationContext::new(program, program_hash);

        // Create dummy proof and params
        let proof = vec![5, 6, 7, 8];
        let input = vec![9, 10, 11, 12];

        let params = VerificationParams {
            proof: &proof,
            input: &input,
            from_chain: 1,
            to_chain: 2,
            nonce: 0,
            timestamp: 0,
        };

        // Test verification
        let result = verify_proof(&context, &params).await;
        assert!(result.is_err()); // Should fail with dummy data
    }
} 