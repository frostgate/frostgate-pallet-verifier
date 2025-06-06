use sp_std::prelude::*;
use sp_runtime::traits::Hash;
use codec::{Decode, Encode};
use frostgate_circuits::sp1::{
    types::{Sp1ProofType, Sp1Backend},
    verifier::verify_proof as sp1_verify_proof,
};

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
    /// SP1 verification error
    Sp1Error(Vec<u8>),
}

/// Result type for verification operations
pub type VerificationResult = Result<(), VerificationError>;

/// Proof verification context
#[derive(Clone)]
pub struct VerificationContext {
    /// Verification key bytes
    pub verifying_key: Vec<u8>,
    /// Program hash
    pub program_hash: [u8; 32],
}

/// Proof verification parameters
pub struct VerificationParams<'a> {
    /// Message proof bytes
    pub proof: &'a [u8],
    /// Message payload
    pub payload: &'a [u8],
    /// Source chain ID
    pub from_chain: u64,
    /// Destination chain ID
    pub to_chain: u64,
    /// Message nonce
    pub nonce: u64,
    /// Message timestamp
    pub timestamp: u64,
}

/// Verify a proof using SP1
pub fn verify_proof(
    context: &VerificationContext,
    params: &VerificationParams,
) -> VerificationResult {
    // Validate input format
    if params.proof.is_empty() {
        return Err(VerificationError::InvalidProofFormat);
    }

    // Check chain IDs are valid
    if params.from_chain > 2 || params.to_chain > 2 {
        return Err(VerificationError::InvalidInput);
    }

    // Check timestamp is reasonable
    if params.timestamp < 1600000000 || params.timestamp > 2000000000 {
        return Err(VerificationError::InvalidInput);
    }

    // Deserialize SP1 proof
    let sp1_proof = match bincode::deserialize::<Sp1ProofType>(params.proof) {
        Ok(p) => p,
        Err(_) => return Err(VerificationError::InvalidProofFormat),
    };

    // Create input vector for verification
    let mut input = Vec::new();
    
    // Add chain IDs
    input.extend_from_slice(&params.from_chain.to_be_bytes());
    input.extend_from_slice(&params.to_chain.to_be_bytes());
    
    // Add payload length and payload
    let payload_len = params.payload.len() as u64;
    input.extend_from_slice(&payload_len.to_be_bytes());
    input.extend_from_slice(params.payload);
    
    // Add nonce and timestamp
    input.extend_from_slice(&params.nonce.to_be_bytes());
    input.extend_from_slice(&params.timestamp.to_be_bytes());

    // Create SP1 backend for verification
    let backend = Sp1Backend::Local(sp1_sdk::EnvProver::new());

    // Verify the proof
    match sp1_verify_proof(&backend, &sp1_proof, &context.verifying_key) {
        Ok(true) => Ok(()),
        Ok(false) => Err(VerificationError::VerificationFailed),
        Err(e) => Err(VerificationError::Sp1Error(e.to_string().into_bytes())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_verification() {
        let context = VerificationContext {
            verifying_key: vec![1, 2, 3], // Mock key
            program_hash: [0; 32],
        };

        // Create a mock SP1 proof
        let mock_proof = bincode::serialize(&Sp1ProofType::Core(
            sp1_sdk::SP1ProofWithPublicValues::default()
        )).unwrap();

        let params = VerificationParams {
            proof: &mock_proof,
            payload: &[4, 5, 6],
            from_chain: 0,
            to_chain: 1,
            nonce: 1,
            timestamp: 1700000000,
        };

        // This will fail because we're using a mock proof
        assert!(verify_proof(&context, &params).is_err());
    }

    #[test]
    fn test_invalid_proof() {
        let context = VerificationContext {
            verifying_key: vec![1, 2, 3],
            program_hash: [0; 32],
        };

        let params = VerificationParams {
            proof: &[], // Empty proof
            payload: &[4, 5, 6],
            from_chain: 0,
            to_chain: 1,
            nonce: 1,
            timestamp: 1700000000,
        };

        assert_eq!(
            verify_proof(&context, &params),
            Err(VerificationError::InvalidProofFormat)
        );
    }

    #[test]
    fn test_invalid_chain_id() {
        let context = VerificationContext {
            verifying_key: vec![1, 2, 3],
            program_hash: [0; 32],
        };

        // Create a mock SP1 proof
        let mock_proof = bincode::serialize(&Sp1ProofType::Core(
            sp1_sdk::SP1ProofWithPublicValues::default()
        )).unwrap();

        let params = VerificationParams {
            proof: &mock_proof,
            payload: &[4, 5, 6],
            from_chain: 3, // Invalid chain ID
            to_chain: 1,
            nonce: 1,
            timestamp: 1700000000,
        };

        assert_eq!(
            verify_proof(&context, &params),
            Err(VerificationError::InvalidInput)
        );
    }
} 