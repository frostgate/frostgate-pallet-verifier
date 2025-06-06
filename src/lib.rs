#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

pub mod verification;
pub mod keys;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::{Currency, ReservableCurrency},
        transactional,
    };
    use frame_system::pallet_prelude::*;
    use sp_std::prelude::*;
    use codec::{Decode, Encode};
    use scale_info::TypeInfo;
    use crate::{
        verification::{VerificationContext, VerificationParams, verify_proof, VerificationError},
        keys::{VerificationKeyEntry, ProgramCacheEntry},
    };

    /// Chain identifier type
    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum ChainId {
        Ethereum = 0,
        Polkadot = 1,
        Solana = 2,
        Unknown = 255,
    }

    impl Default for ChainId {
        fn default() -> Self {
            ChainId::Unknown
        }
    }

    /// Message status
    #[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
    pub enum MessageStatus {
        Pending,
        Verified,
        Failed,
    }

    impl Default for MessageStatus {
        fn default() -> Self {
            MessageStatus::Pending
        }
    }

    /// Message data stored on-chain
    #[derive(Clone, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo)]
    pub struct Message<AccountId> {
        from_chain: ChainId,
        to_chain: ChainId,
        sender: AccountId,
        payload: Vec<u8>,
        nonce: u64,
        timestamp: u64,
        status: MessageStatus,
        proof: Option<Vec<u8>>,
    }

    /// Configuration trait for the pallet
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Currency type for fees and deposits
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Maximum size of message payload
        #[pallet::constant]
        type MaxPayloadSize: Get<u32>;

        /// Required deposit for submitting a message
        #[pallet::constant]
        type MessageDeposit: Get<BalanceOf<Self>>;

        /// Maximum size of verification key
        #[pallet::constant]
        type MaxKeySize: Get<u32>;

        /// Maximum age of cached programs (in blocks)
        #[pallet::constant]
        type MaxProgramAge: Get<u32>;
    }

    type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Message storage - maps message hash to message data
    #[pallet::storage]
    pub type Messages<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        Message<T::AccountId>,
        OptionQuery,
    >;

    /// Nonce storage - per chain ID and account
    #[pallet::storage]
    pub type Nonces<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        ChainId,
        Blake2_128Concat,
        T::AccountId,
        u64,
        ValueQuery,
    >;

    /// Verification key storage - maps program hash to verification key
    #[pallet::storage]
    pub type VerificationKeys<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        VerificationKeyEntry,
        OptionQuery,
    >;

    /// Program cache storage - maps program hash to program data
    #[pallet::storage]
    pub type ProgramCache<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        ProgramCacheEntry,
        OptionQuery,
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// A new message was submitted
        MessageSubmitted {
            hash: T::Hash,
            from_chain: ChainId,
            to_chain: ChainId,
            sender: T::AccountId,
        },
        /// A message was verified successfully
        MessageVerified {
            hash: T::Hash,
            from_chain: ChainId,
            to_chain: ChainId,
        },
        /// Message verification failed
        MessageVerificationFailed {
            hash: T::Hash,
            error: Vec<u8>,
        },
        /// New verification key added
        VerificationKeyAdded {
            program_hash: [u8; 32],
        },
        /// Program cached
        ProgramCached {
            program_hash: [u8; 32],
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Message payload too large
        PayloadTooLarge,
        /// Message not found
        MessageNotFound,
        /// Invalid chain ID
        InvalidChainId,
        /// Invalid proof format
        InvalidProof,
        /// Message already verified
        AlreadyVerified,
        /// Invalid message status transition
        InvalidStatusTransition,
        /// Verification failed
        VerificationFailed,
        /// Verification key too large
        KeyTooLarge,
        /// Invalid verification key
        InvalidKey,
        /// Program not found
        ProgramNotFound,
        /// SP1 verification error
        Sp1Error(Vec<u8>),
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Submit a new message for verification
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        #[transactional]
        pub fn submit_message(
            origin: OriginFor<T>,
            from_chain: ChainId,
            to_chain: ChainId,
            payload: Vec<u8>,
            proof: Option<Vec<u8>>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;

            // Validate inputs
            ensure!(payload.len() <= T::MaxPayloadSize::get() as usize, Error::<T>::PayloadTooLarge);
            ensure!(from_chain != ChainId::Unknown && to_chain != ChainId::Unknown, Error::<T>::InvalidChainId);

            // Get and increment nonce
            let nonce = Self::get_next_nonce(from_chain, &sender);

            // Create message
            let message = Message {
                from_chain,
                to_chain,
                sender: sender.clone(),
                payload,
                nonce,
                timestamp: T::BlockNumber::current().saturated_into::<u64>(),
                status: MessageStatus::Pending,
                proof,
            };

            // Generate message hash
            let hash = T::Hashing::hash_of(&message);

            // Reserve deposit
            T::Currency::reserve(&sender, T::MessageDeposit::get())?;

            // Store message
            Messages::<T>::insert(hash, message);

            // Emit event
            Self::deposit_event(Event::MessageSubmitted {
                hash,
                from_chain,
                to_chain,
                sender,
            });

            Ok(())
        }

        /// Verify a submitted message
        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn verify_message(
            origin: OriginFor<T>,
            message_hash: T::Hash,
        ) -> DispatchResult {
            ensure_signed(origin)?;

            // Get message
            let mut message = Messages::<T>::get(message_hash)
                .ok_or(Error::<T>::MessageNotFound)?;

            // Check status
            ensure!(message.status == MessageStatus::Pending, Error::<T>::InvalidStatusTransition);

            // Get proof and verify
            if let Some(proof) = &message.proof {
                // Get verification key for the program
                let program_hash = Self::compute_program_hash(&message);
                let key_entry = VerificationKeys::<T>::get(program_hash)
                    .ok_or(Error::<T>::InvalidKey)?;

                // Create verification context
                let context = VerificationContext {
                    verifying_key: key_entry.key_bytes,
                    program_hash,
                };

                // Create verification params
                let params = VerificationParams {
                    proof,
                    payload: &message.payload,
                    from_chain: message.from_chain as u64,
                    to_chain: message.to_chain as u64,
                    nonce: message.nonce,
                    timestamp: message.timestamp,
                };

                // Verify proof
                match verify_proof(&context, &params) {
                    Ok(()) => {
                        // Update status
                        message.status = MessageStatus::Verified;
                        Messages::<T>::insert(message_hash, message.clone());

                        // Emit event
                        Self::deposit_event(Event::MessageVerified {
                            hash: message_hash,
                            from_chain: message.from_chain,
                            to_chain: message.to_chain,
                        });
                    }
                    Err(e) => {
                        // Update status to failed
                        message.status = MessageStatus::Failed;
                        Messages::<T>::insert(message_hash, message);

                        // Convert error and emit event
                        let error_bytes = match e {
                            VerificationError::InvalidProofFormat => b"Invalid proof format".to_vec(),
                            VerificationError::VerificationFailed => b"Verification failed".to_vec(),
                            VerificationError::InvalidInput => b"Invalid input".to_vec(),
                            VerificationError::SystemError => b"System error".to_vec(),
                            VerificationError::Sp1Error(bytes) => bytes,
                        };

                        Self::deposit_event(Event::MessageVerificationFailed {
                            hash: message_hash,
                            error: error_bytes.clone(),
                        });

                        // Map error type to pallet error
                        match e {
                            VerificationError::InvalidProofFormat => return Err(Error::<T>::InvalidProof.into()),
                            VerificationError::VerificationFailed => return Err(Error::<T>::VerificationFailed.into()),
                            VerificationError::InvalidInput => return Err(Error::<T>::InvalidChainId.into()),
                            VerificationError::SystemError => return Err(Error::<T>::VerificationFailed.into()),
                            VerificationError::Sp1Error(_) => return Err(Error::<T>::Sp1Error(error_bytes).into()),
                        }
                    }
                }
            }

            Ok(())
        }

        /// Add or update a verification key
        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn add_verification_key(
            origin: OriginFor<T>,
            program_hash: [u8; 32],
            key_bytes: Vec<u8>,
            metadata: Option<Vec<u8>>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            // Validate key size
            ensure!(key_bytes.len() <= T::MaxKeySize::get() as usize, Error::<T>::KeyTooLarge);

            // Create key entry
            let key_entry = VerificationKeyEntry::new(
                program_hash,
                key_bytes,
                T::BlockNumber::current().saturated_into::<u64>(),
                metadata,
            );

            // Validate key format
            key_entry.validate().map_err(|_| Error::<T>::InvalidKey)?;

            // Store key
            VerificationKeys::<T>::insert(program_hash, key_entry);

            // Emit event
            Self::deposit_event(Event::VerificationKeyAdded {
                program_hash,
            });

            Ok(())
        }

        /// Cache a program for verification
        #[pallet::call_index(3)]
        #[pallet::weight(10_000)]
        pub fn cache_program(
            origin: OriginFor<T>,
            program_hash: [u8; 32],
            program_bytes: Vec<u8>,
        ) -> DispatchResult {
            ensure_root(origin)?;

            // Create cache entry
            let entry = ProgramCacheEntry::new(
                program_hash,
                program_bytes,
                T::BlockNumber::current().saturated_into::<u64>(),
            );

            // Store program
            ProgramCache::<T>::insert(program_hash, entry);

            // Emit event
            Self::deposit_event(Event::ProgramCached {
                program_hash,
            });

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        /// Get and increment nonce for a chain and account
        fn get_next_nonce(chain_id: ChainId, account: &T::AccountId) -> u64 {
            let nonce = Nonces::<T>::get(chain_id, account);
            Nonces::<T>::insert(chain_id, account, nonce + 1);
            nonce
        }

        /// Compute program hash for a message
        fn compute_program_hash(message: &Message<T::AccountId>) -> [u8; 32] {
            // TODO: Implement proper program hash computation
            // For now, use a dummy hash based on chain IDs
            let mut hash = [0u8; 32];
            hash[0] = message.from_chain as u8;
            hash[1] = message.to_chain as u8;
            hash
        }

        /// Clean up old program cache entries
        pub(crate) fn cleanup_program_cache() {
            let current_block = T::BlockNumber::current().saturated_into::<u64>();
            let max_age = T::MaxProgramAge::get() as u64;

            ProgramCache::<T>::retain(|_, entry| {
                current_block.saturating_sub(entry.cached_at) < max_age
            });
        }
    }
} 