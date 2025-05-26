//! The [`middleware`] module provides functionality to interact with
//! Ethereum-like virtual machines. It achieves this by offering a middleware
//! implementation for sending and reading transactions, as well as watching
//! for events.
//!
//! Main components:
//! - [`ArbiterMiddleware`]: The core middleware implementation.
//! - [`Connection`]: Handles communication with the Ethereum VM.
//! - [`FilterReceiver`]: Facilitates event watching based on certain filters.

#![warn(missing_docs)]
use std::{future::Future, pin::Pin, sync::Mutex, time::Duration};

use futures_timer::Delay;
use futures_util::Stream;
use serde::de::DeserializeOwned;
use serde_json::value::RawValue;
use sha2::Sha256;
use starknet::{
    core::{
        crypto::Signature,
        types::{BlockId, BroadcastedTransaction, Felt, FunctionCall, TransactionWithReceipt},
    },
    providers::{JsonRpcClient, ProviderError},
    signers::{LocalWallet, Signer, SignerInteractivityContext, SigningKey, VerifyingKey},
};

use super::*;
use crate::environment::{instruction::*, Broadcast, Environment};

pub mod connection;
use connection::*;

pub mod nonce_middleware;
/// A middleware structure that integrates with `revm`.
///
/// [`ArbiterMiddleware`] serves as a bridge between the application and
/// [`revm`]'s execution environment, allowing for transaction sending, call
/// execution, and other core functions. It uses a custom connection and error
/// system tailored to Revm's specific needs.
///
/// This allows for [`revm`] and the [`Environment`] built around it to be
/// treated in much the same way as a live EVM blockchain can be addressed.
///
/// # Examples
///
/// Basic usage:
/// ```
/// use arbiter_core::{environment::Environment, middleware::ArbiterMiddleware};
///
/// // Create a new environment and run it
/// let mut environment = Environment::builder().build();
///
/// // Retrieve the environment to create a new middleware instance
/// let middleware = ArbiterMiddleware::new(&environment, Some("test_label"));
/// ```
/// The client can now be used for transactions with the environment.
/// Use a seed like `Some("test_label")` for maintaining a
/// consistent address across simulations and client labeling. Seeding is be
/// useful for debugging and post-processing.
#[derive(Debug)]
pub struct ArbiterMiddleware {
    connection: Connection,
    wallet: EOA,
    /// An optional label for the middleware instance
    #[allow(unused)]
    pub label: Option<String>,
}

#[async_trait]
impl Signer for ArbiterMiddleware {
    type GetPublicKeyError = ArbiterCoreError;
    type SignError = ArbiterCoreError;

    async fn get_public_key(&self) -> Result<VerifyingKey, Self::GetPublicKeyError> {
        match self.wallet {
            // TODO: new error maybe?
            EOA::Forked(_) => Err(ArbiterCoreError::ForkedEOASignError),
            EOA::Wallet(wallet) => wallet.get_public_key(),
        }
    }

    async fn sign_hash(&self, hash: &Felt) -> Result<Signature, Self::SignError> {
        match self.wallet {
            EOA::Forked(_) => Err(ArbiterCoreError::ForkedEOASignError),
            EOA::Wallet(wallet) => wallet.sign_hash(hash),
        }
    }

    fn is_interactive(&self, context: SignerInteractivityContext<'_>) -> bool {
        return true;
    }
}

/// A wrapper enum for the two types of accounts that can be used with the
/// middleware.
#[derive(Debug, Clone)]
pub enum EOA {
    /// The [`Forked`] variant is used for the forked EOA,
    /// allowing us to treat them as mock accounts that we can still authorize
    /// transactions with that we would be unable to do on mainnet.
    Forked(eAddress),
    /// The [`Wallet`] variant "real" in the sense that is has a valid private
    /// key from the provided seed
    Wallet(LocalWallet),
}

impl ArbiterMiddleware {
    pub fn new(
        environment: &Environment,
        seed_and_label: Option<&str>,
    ) -> Result<Arc<Self>, ArbiterCoreError> {
        let connection = Connection::from(environment);

        let signing_key = if let Some(seed) = seed_and_label {
            // seed_and_label
            let mut hasher = Sha256::new();
            hasher.update(seed);
            let hashed = hasher.finalize();
            // TODO: probably will fail
            SigningKey::from_secret_scalar(hashed.into())
        } else {
            SigningKey::from_random()
        };

        let wallet = LocalWallet::from_signing_key(signing_key);

        connection
            .instruction_sender
            .upgrade() // TODO: WHY?!
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::AddAccount {
                address: wallet.address(),
                outcome_sender: connection.outcome_sender.clone(),
            })?;

        connection.outcome_receiver.recv()??;
        info!(
            "Created new `ArbiterMiddleware` instance from a fork -- attached to environment labeled: {:?}",
            environment.parameters.label
        );
        Ok(Arc::new(Self {
            wallet: EOA::Wallet(wallet),
            connection,
            label: seed_and_label.map(|s| s.to_string()),
        }))
    }

    // TODO: This needs to have the label retrieved from the fork config.
    /// Creates a new instance of `ArbiterMiddleware` from a forked EOA.
    pub fn new_from_forked_eoa(
        environment: &Environment,
        forked_eoa: eAddress,
    ) -> Result<Arc<Self>, ArbiterCoreError> {
        let instruction_sender = &Arc::clone(&environment.socket.instruction_sender);
        let (outcome_sender, outcome_receiver) = crossbeam_channel::unbounded();

        let connection = Connection {
            instruction_sender: Arc::downgrade(instruction_sender),
            outcome_sender,
            outcome_receiver: outcome_receiver.clone(),
            event_sender: environment.socket.event_broadcaster.clone(),
            filter_receivers: Arc::new(Mutex::new(HashMap::new())),
        };
        info!(
            "Created new `ArbiterMiddleware` instance from a fork -- attached to environment labeled: {:?}",
            environment.parameters.label
        );
        Ok(Arc::new(Self {
            wallet: EOA::Forked(forked_eoa),
            connection,
            label: None,
        }))
    }

    /// Provides access to the associated Ethereum provider which is given by
    /// the [`Provider<Connection>`] for [`ArbiterMiddleware`].
    fn provider(&self) -> Connection {
        &self.connection
    }

    /// Allows the user to update the block number and timestamp of the
    /// [`Environment`] to whatever they may choose at any time.
    pub fn update_block(
        &self,
        block_number: impl Into<eU256>,
        block_timestamp: impl Into<eU256>,
    ) -> Result<ReceiptData, ArbiterCoreError> {
        let provider = self.provider().as_ref();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::BlockUpdate {
                block_number: block_number.into(),
                block_timestamp: block_timestamp.into(),
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::BlockUpdateCompleted(receipt_data) => Ok(receipt_data),
            _ => unreachable!(),
        }
    }

    /// Returns the timestamp of the current block.
    pub async fn get_block_timestamp(&self) -> Result<eU256, ArbiterCoreError> {
        let provider = self.provider().as_ref();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Query {
                environment_data: EnvironmentData::BlockTimestamp,
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::QueryReturn(outcome) => Ok(eU256::from_str_radix(outcome.as_ref(), 10)?),
            _ => unreachable!(),
        }
    }

    /// Sends a cheatcode instruction to the environment.
    pub async fn apply_cheatcode(
        &self,
        cheatcode: Cheatcodes,
    ) -> Result<CheatcodesReturn, ArbiterCoreError> {
        let provider = self.provider.as_ref();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Cheatcode {
                cheatcode,
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::CheatcodeReturn(outcome) => Ok(outcome),
            _ => unreachable!(),
        }
    }

    /// Returns the address of the wallet/signer given to a client.
    /// Matches on the [`EOA`] variant of the [`ArbiterMiddleware`] struct.
    pub fn address(&self) -> eAddress {
        match &self.wallet {
            EOA::Forked(address) => *address,
            EOA::Wallet(wallet) => wallet.address(),
        }
    }

    /// Allows a client to set a gas price for transactions.
    /// This can only be done if the [`Environment`] has
    /// [`EnvironmentParameters`] `gas_settings` field set to
    /// [`GasSettings::UserControlled`].
    pub async fn set_gas_price(&self, gas_price: eU256) -> Result<(), ArbiterCoreError> {
        let provider = self.provider.as_ref();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::SetGasPrice {
                gas_price,
                outcome_sender: provider.outcome_sender.clone(),
            })?;
        match provider.outcome_receiver.recv()?? {
            Outcome::SetGasPriceCompleted => {
                debug!("Gas price set");
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    /// Returns a reference to the inner middleware of which there is none when
    /// using [`ArbiterMiddleware`] so we relink to `Self`
    fn inner(&self) -> &Self::Inner {
        &self.provider
    }

    /// Provides the default sender address for transactions, i.e., the address
    /// of the wallet/signer given to a client of the [`Environment`].
    fn default_sender(&self) -> Option<eAddress> {
        Some(self.address())
    }

    /// Sends a transaction to the [`Environment`] which acts as a simulated
    /// Ethereum network.
    ///
    /// The method checks if the transaction is either a call to an existing
    /// contract or a deploy of a new one, and constructs the necessary
    /// transaction environment used for `revm`-based transactions.
    /// It then sends this transaction for execution and returns the
    /// corresponding pending transaction.
    async fn send_transaction<T: Into<BroadcastedTransaction> + Send + Sync>(
        &self,
        tx: T,
        _block: Option<BlockId>,
    ) -> Result<TransactionWithReceipt<'_, Self::Provider>, Self::Error> {
        trace!("Building transaction");

        // Check the `to` field of the transaction to determine if it is a call or a
        // deploy. If there is no `to` field, then it is a `Deploy` else it is a
        // `Call`.

        let instruction = Instruction::Transaction {
            tx: tx.into(),
            outcome_sender: self.provider.as_ref().outcome_sender.clone(),
        };

        let provider = self.provider.as_ref();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(instruction)?;

        let outcome = provider.outcome_receiver.recv()??;

        if let Outcome::TransactionCompleted(execution_result, receipt_data) = outcome {
            match execution_result {
                ExecutionResult::Revert { gas_used, output } => {
                    return Err(ArbiterCoreError::ExecutionRevert {
                        gas_used,
                        output: output.to_vec(),
                    });
                }
                ExecutionResult::Halt { reason, gas_used } => {
                    return Err(ArbiterCoreError::ExecutionHalt { reason, gas_used });
                }
                ExecutionResult::Success {
                    output,
                    gas_used,
                    logs,
                    ..
                } => {
                    // TODO: This is why we need the signer middleware
                    // Note that this is technically not the correct construction on the tx hash
                    // but until we increment the nonce correctly this will do
                    let sender = self.address();

                    let logs = revm_logs_to_ethers_logs(logs, &receipt_data);
                    let to: Option<eAddress> = match tx_env.transact_to {
                        TransactTo::Call(address) => Some(address.into_array().into()),
                        TransactTo::Create(_) => None,
                    };

                    match output {
                        Output::Create(_, address) => {
                            let tx_receipt = TransactionReceipt {
                                block_hash: None,
                                block_number: Some(receipt_data.block_number),
                                contract_address: Some(recast_address(address.unwrap())),
                                logs: logs.clone(),
                                from: sender,
                                gas_used: Some(gas_used.into()),
                                effective_gas_price: Some(
                                    tx_env.clone().gas_price.to_be_bytes().into(),
                                ),
                                transaction_hash: H256::default(),
                                to,
                                cumulative_gas_used: receipt_data.cumulative_gas_per_block,
                                status: Some(1.into()),
                                root: None,
                                logs_bloom: {
                                    let mut bloom = Bloom::default();
                                    for log in &logs {
                                        bloom.accrue(BloomInput::Raw(&log.address.0));
                                        for topic in log.topics.iter() {
                                            bloom.accrue(BloomInput::Raw(topic.as_bytes()));
                                        }
                                    }
                                    bloom
                                },
                                transaction_type: match tx {
                                    TypedTransaction::Eip2930(_) => Some(1.into()),
                                    _ => None,
                                },
                                transaction_index: receipt_data.transaction_index,
                                ..Default::default()
                            };

                            // TODO: I'm not sure we need to set the confirmations.
                            let mut pending_tx = PendingTransaction::new(
                                ethers::types::H256::zero(),
                                self.provider(),
                            )
                            .interval(Duration::ZERO)
                            .confirmations(0);

                            let state_ptr: *mut PendingTxState =
                                &mut pending_tx as *mut _ as *mut PendingTxState;

                            // Modify the value (this assumes you have access to the enum variants)
                            unsafe {
                                *state_ptr = PendingTxState::CheckingReceipt(Some(tx_receipt));
                            }

                            Ok(pending_tx)
                        }
                        Output::Call(_) => {
                            let tx_receipt = TransactionReceipt {
                                block_hash: None,
                                block_number: Some(receipt_data.block_number),
                                contract_address: None,
                                logs: logs.clone(),
                                from: sender,
                                gas_used: Some(gas_used.into()),
                                effective_gas_price: Some(
                                    tx_env.clone().gas_price.to_be_bytes().into(),
                                ),
                                transaction_hash: H256::default(),
                                to,
                                cumulative_gas_used: receipt_data.cumulative_gas_per_block,
                                status: Some(1.into()),
                                root: None,
                                logs_bloom: {
                                    let mut bloom = Bloom::default();
                                    for log in &logs {
                                        bloom.accrue(BloomInput::Raw(&log.address.0));
                                        for topic in log.topics.iter() {
                                            bloom.accrue(BloomInput::Raw(topic.as_bytes()));
                                        }
                                    }
                                    bloom
                                },
                                transaction_type: match tx {
                                    TypedTransaction::Eip2930(_) => Some(1.into()),
                                    _ => None,
                                },
                                transaction_index: receipt_data.transaction_index,
                                ..Default::default()
                            };

                            let mut pending_tx = PendingTransaction::new(
                                ethers::types::H256::zero(),
                                self.provider(),
                            )
                            .interval(Duration::ZERO)
                            .confirmations(0);

                            let state_ptr: *mut PendingTxState =
                                &mut pending_tx as *mut _ as *mut PendingTxState;

                            // Modify the value (this assumes you have access to the enum variants)
                            unsafe {
                                *state_ptr = PendingTxState::CheckingReceipt(Some(tx_receipt));
                            }

                            Ok(pending_tx)
                        }
                    }
                }
            }
        } else {
            unreachable!()
        }
    }

    /// Calls a contract method without creating a worldstate-changing
    /// transaction on the [`Environment`] (again, simulating the Ethereum
    /// network).
    ///
    /// Similar to `send_transaction`, this method checks if the call is
    /// targeting an existing contract or deploying a new one. After
    /// executing the call, it returns the output, but no worldstate change will
    /// be documented in the `revm` DB.
    async fn call(
        &self,
        call: &FunctionCall,
        _block: Option<BlockId>,
    ) -> Result<eBytes, Self::Error> {
        trace!("Building call");

        let instruction = Instruction::Call {
            call,
            outcome_sender: self.provider().as_ref().outcome_sender.clone(),
        };
        self.provider()
            .as_ref()
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(instruction)?;

        let outcome = self.provider().as_ref().outcome_receiver.recv()??;

        if let Outcome::CallCompleted(execution_result) = outcome {
            match execution_result {
                ExecutionResult::Revert { gas_used, output } => {
                    return Err(ArbiterCoreError::ExecutionRevert {
                        gas_used,
                        output: output.to_vec(),
                    });
                }
                ExecutionResult::Halt { reason, gas_used } => {
                    return Err(ArbiterCoreError::ExecutionHalt { reason, gas_used });
                }
                ExecutionResult::Success { output, .. } => {
                    return Ok(eBytes::from(output.data().to_vec()));
                }
            }
        } else {
            unreachable!()
        }
    }

    async fn get_gas_price(&self) -> Result<eU256, Self::Error> {
        let provider = self.provider.as_ref();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Query {
                environment_data: EnvironmentData::GasPrice,
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::QueryReturn(outcome) => Ok(eU256::from_str_radix(outcome.as_ref(), 10)?),
            _ => unreachable!(),
        }
    }

    async fn get_block_number(&self) -> Result<u64, Self::Error> {
        let provider = self.provider().as_ref();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Query {
                environment_data: EnvironmentData::BlockNumber,
                outcome_sender: provider.outcome_sender.clone(),
            })?;
        match provider.outcome_receiver.recv()?? {
            Outcome::QueryReturn(outcome) => Ok(u64::from_str_radix(outcome.as_ref(), 10)?),
            _ => unreachable!(),
        }
    }

    async fn get_balance<T: Into<eAddress> + Send + Sync>(
        &self,
        from: T,
        block: Option<BlockId>,
    ) -> Result<eU256, Self::Error> {
        if block.is_some() {
            return Err(ArbiterCoreError::InvalidQueryError);
        }
        let address: eAddress = from.into();

        let provider = self.provider.as_ref();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Query {
                environment_data: EnvironmentData::Balance(address),
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::QueryReturn(outcome) => Ok(eU256::from_str_radix(outcome.as_ref(), 10)?),
            _ => unreachable!(),
        }
    }

    /// Returns the nonce of the address
    async fn get_transaction_count<T: Into<eAddress> + Send + Sync>(
        &self,
        from: T,
        _block: Option<BlockId>,
    ) -> Result<eU256, Self::Error> {
        let address: eAddress = from.into();
        let provider = self.provider.as_ref();
        provider
            .instruction_sender
            .upgrade()
            .ok_or(ArbiterCoreError::UpgradeSenderError)?
            .send(Instruction::Query {
                environment_data: EnvironmentData::TransactionCount(address),
                outcome_sender: provider.outcome_sender.clone(),
            })?;

        match provider.outcome_receiver.recv()?? {
            Outcome::QueryReturn(outcome) => Ok(eU256::from_str_radix(outcome.as_ref(), 10)?),
            _ => unreachable!(),
        }
    }

    /// Fill necessary details of a transaction for dispatch
    ///
    /// This function is defined on providers to behave as follows:
    /// 1. populate the `from` field with the client address
    /// 2. Estimate gas usage
    ///
    /// It does NOT set the nonce by default.

    async fn fill_transaction(
        &self,
        tx: &mut BroadcastedTransaction,
        _block: Option<BlockId>,
    ) -> Result<(), Self::Error> {
        // Set the `from` field of the transaction to the client address
        if tx.from().is_none() {
            tx.set_from(self.address());
        }

        // get the gas usage price
        if tx.gas_price().is_none() {
            let gas_price = self.get_gas_price().await?;
            tx.set_gas_price(gas_price);
        }

        Ok(())
    }
    /// Fetches the value stored at the storage slot `key` for an account at
    /// `address`. todo: implement the storage at a specific block feature.
    async fn get_storage_at<T: Into<eAddress> + Send + Sync>(
        &self,
        account: T,
        key: H256,
        block: Option<BlockId>,
    ) -> Result<H256, ArbiterCoreError> {
        let address: eAddress = account.into();

        let result = self
            .apply_cheatcode(Cheatcodes::Load {
                account: address,
                key,
                block,
            })
            .await
            .unwrap();

        match result {
            CheatcodesReturn::Load { value } => {
                // Convert the revm ruint type into big endian bytes, then convert into ethers
                // H256.
                let value: H256 = H256::from(value.to_be_bytes());
                Ok(value)
            }
            _ => unreachable!(),
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) type PinBoxFut<'a, T> = Pin<Box<dyn Future<Output = Result<T, ProviderError>> + 'a>>;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) type PinBoxFut<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ProviderError>> + Send + 'a>>;

// Because this is the exact same struct it will have the exact same memory
// alignment allowing us to bypass the fact that ethers-rs doesn't export this
// enum normally We box the TransactionReceipts to keep the enum small.
#[allow(unused, missing_docs)]
pub enum PendingTxState<'a> {
    /// Initial delay to ensure the GettingTx loop doesn't immediately fail
    InitialDelay(Pin<Box<Delay>>),

    /// Waiting for interval to elapse before calling API again
    PausedGettingTx,

    /// Polling The blockchain to see if the Tx has confirmed or dropped
    GettingTx(PinBoxFut<'a, Option<Transaction>>),

    /// Waiting for interval to elapse before calling API again
    PausedGettingReceipt,

    /// Polling the blockchain for the receipt
    GettingReceipt(PinBoxFut<'a, Option<TransactionReceipt>>),

    /// If the pending tx required only 1 conf, it will return early. Otherwise
    /// it will proceed to the next state which will poll the block number
    /// until there have been enough confirmations
    CheckingReceipt(Option<TransactionReceipt>),

    /// Waiting for interval to elapse before calling API again
    PausedGettingBlockNumber(Option<TransactionReceipt>),

    /// Polling the blockchain for the current block number
    GettingBlockNumber(PinBoxFut<'a, U64>, Option<TransactionReceipt>),

    /// Future has completed and should panic if polled again
    Completed,
}
