use sp_std::prelude::*;
use codec::{Decode, Encode};
use sp_runtime::RuntimeDebug;
use scale_info::TypeInfo;

/// Key management error types
#[derive(Debug, Encode, Decode, PartialEq, Eq)]
pub enum KeyError {
    /// Invalid key format
    InvalidFormat,
    /// Key not found
    NotFound,
    /// Invalid program hash
    InvalidProgramHash,
    /// System error
    SystemError,
}

/// Result type for key operations
pub type KeyResult<T> = Result<T, KeyError>;

/// Verification key entry stored on-chain
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct VerificationKeyEntry {
    /// Program hash this key is for
    pub program_hash: [u8; 32],
    /// Verification key bytes
    pub key_bytes: Vec<u8>,
    /// Block number when this key was added
    pub added_at: u64,
    /// Optional metadata
    pub metadata: Option<Vec<u8>>,
}

impl VerificationKeyEntry {
    /// Create a new verification key entry
    pub fn new(
        program_hash: [u8; 32],
        key_bytes: Vec<u8>,
        added_at: u64,
        metadata: Option<Vec<u8>>,
    ) -> Self {
        Self {
            program_hash,
            key_bytes,
            added_at,
            metadata,
        }
    }

    /// Validate the key entry format
    pub fn validate(&self) -> KeyResult<()> {
        // Check key is not empty
        if self.key_bytes.is_empty() {
            return Err(KeyError::InvalidFormat);
        }

        // Check program hash is not zero
        if self.program_hash.iter().all(|&x| x == 0) {
            return Err(KeyError::InvalidProgramHash);
        }

        Ok(())
    }
}

/// Program cache entry
#[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
pub struct ProgramCacheEntry {
    /// Program hash
    pub hash: [u8; 32],
    /// Program bytes
    pub bytes: Vec<u8>,
    /// Block number when cached
    pub cached_at: u64,
    /// Number of times used
    pub use_count: u64,
}

impl ProgramCacheEntry {
    /// Create a new program cache entry
    pub fn new(
        hash: [u8; 32],
        bytes: Vec<u8>,
        cached_at: u64,
    ) -> Self {
        Self {
            hash,
            bytes,
            cached_at,
            use_count: 0,
        }
    }

    /// Increment use count
    pub fn increment_use_count(&mut self) {
        self.use_count = self.use_count.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_key_validation() {
        let valid_key = VerificationKeyEntry::new(
            [1; 32],
            vec![1, 2, 3],
            1,
            None,
        );
        assert!(valid_key.validate().is_ok());

        let invalid_key = VerificationKeyEntry::new(
            [0; 32],
            vec![],
            1,
            None,
        );
        assert!(invalid_key.validate().is_err());
    }

    #[test]
    fn test_program_cache() {
        let mut entry = ProgramCacheEntry::new(
            [1; 32],
            vec![1, 2, 3],
            1,
        );
        assert_eq!(entry.use_count, 0);

        entry.increment_use_count();
        assert_eq!(entry.use_count, 1);
    }
} 