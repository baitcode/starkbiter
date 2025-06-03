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
use std::{pin::Pin, sync::Mutex};

use async_trait;
use starknet_devnet_types::starknet_api::{core::ContractAddress, crypto::utils::PublicKey};

use super::*;
use crate::environment::{instruction::*, Broadcast, Environment};
use starknet::{
    core::types::{BlockId, Felt},
    providers::{Provider, ProviderError, ProviderRequestData, ProviderResponseData},
    signers::{
        local_wallet::SignError, LocalWallet, Signer, SignerInteractivityContext, SigningKey,
        VerifyingKey,
    },
};
use starknet_core::{
    crypto::Signature,
    types::{self as core_types, EventFilter},
};

pub mod connection;
use connection::*;

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
/// let middleware = ArbiterMiddleware::new(&environment);
/// ```
/// The client can now be used for transactions with the environment.
/// Use a seed like `Some("test_label")` for maintaining a
/// consistent address across simulations and client labeling. Seeding is be
/// useful for debugging and post-processing.
#[derive(Debug)]
pub struct ArbiterMiddleware {
    connection: Connection,

    #[allow(unused)]
    pub label: Option<String>,
}

impl ArbiterMiddleware {
    pub fn new(
        environment: &Environment,
        seed_and_label: Option<&str>,
    ) -> Result<Arc<Self>, ArbiterCoreError> {
        let connection = Connection::from(environment);

        info!(
            "Created new `ArbiterMiddleware` instance from a fork -- attached to environment labeled: {:?}",
            environment.parameters.label
        );
        Ok(Arc::new(Self {
            connection,
            label: seed_and_label.map(|s| s.to_string()),
        }))
    }

    pub async fn create_account(
        &self,
        public_key: VerifyingKey,
        class_hash: Felt,
    ) -> Result<ContractAddress, ProviderError> {
        return Ok(self
            .connection
            .create_account(public_key, class_hash)
            .await?);
    }

    pub async fn create_block(&self) -> Result<(), ProviderError> {
        return Ok(self.connection.create_block().await?);
    }
}

// #[async_trait::async_trait]
// impl Signer for LocalSigner {
//     type GetPublicKeyError = Infallible;
//     type SignError = SignError;

//     async fn get_public_key(&self) -> Result<VerifyingKey, Self::GetPublicKeyError> {
//         Ok(self.signing_key.verifying_key())
//     }

//     async fn sign_hash(&self, hash: &Felt) -> Result<Signature, Self::SignError> {
//         Ok(self.signing_key.sign(hash)?)
//     }

//     fn is_interactive(&self, _context: SignerInteractivityContext) -> bool {
//         false
//     }
// }

//     pub fn new(
//         environment: &Environment,
//         seed_and_label: Option<Felt>,
//     ) -> Result<Arc<Self>, ArbiterCoreError> {
//         let connection = Connection::from(environment);

//         let signing_key = if let Some(seed) = seed_and_label {
//             // TODO: probably will fail
//             SigningKey::from_secret_scalar(seed)
//         } else {
//             SigningKey::from_random()
//         };

//         let pk = signing_key.verifying_key().scalar();

//         connection
//             .instruction_sender
//             .upgrade() // TODO: WHY?!
//             .ok_or(ArbiterCoreError::UpgradeSenderError)?
//             .send(Instruction::AddAccount {
//                 public_key: pk,
//                 class_hash: ClassHash(constants::ARGENT_CONTRACT_CLASS_HASH),
//                 outcome_sender: connection.outcome_sender.clone(),
//             })?;

//         let address = match connection.outcome_receiver.recv()?? {
//             Outcome::AddAccountCompleted(address) => address,
//             _ => unreachable!(),
//         };

//         info!(
//             "Created new `ArbiterMiddleware` instance from a fork -- attached to environment labeled: {:?}",
//             environment.parameters.label
//         );
//         Ok(Arc::new(Self {
//             account_address: address,
//             account_signing_key: signing_key,
//             connection,
//             label: seed_and_label.map(|s| s.to_string()),
//         }))
//     }

//     /// Provides access to the associated Ethereum provider which is given by
//     /// the [`Provider<Connection>`] for [`ArbiterMiddleware`].
//     fn provider(&self) -> &Connection {
//         &self.connection
//     }

//     /// Allows the user to update the block number and timestamp of the
//     /// [`Environment`] to whatever they may choose at any time.
//     pub fn update_block(
//         &self,
//         block_number: impl Into<BlockNumber>,
//         block_timestamp: impl Into<BlockTimestamp>,
//     ) -> Result<ReceiptData, ArbiterCoreError> {
//         let provider = self.provider();
//         provider
//             .instruction_sender
//             .upgrade()
//             .ok_or(ArbiterCoreError::UpgradeSenderError)?
//             .send(Instruction::BlockUpdate {
//                 block_number: block_number.into(),
//                 block_timestamp: block_timestamp.into(),
//                 outcome_sender: provider.outcome_sender.clone(),
//             })?;

//         match provider.outcome_receiver.recv()?? {
//             Outcome::BlockUpdateCompleted(receipt_data) => Ok(receipt_data),
//             _ => unreachable!(),
//         }
//     }

//     /// Sends a cheatcode instruction to the environment.
//     pub async fn apply_cheatcode(
//         &self,
//         cheatcode: Cheatcodes,
//     ) -> Result<CheatcodesReturn, ArbiterCoreError> {
//         let provider = self.provider();
//         provider
//             .instruction_sender
//             .upgrade()
//             .ok_or(ArbiterCoreError::UpgradeSenderError)?
//             .send(Instruction::Cheatcode {
//                 cheatcode,
//                 outcome_sender: provider.outcome_sender.clone(),
//             })?;

//         match provider.outcome_receiver.recv()?? {
//             Outcome::CheatcodeReturn(outcome) => Ok(outcome),
//             _ => unreachable!(),
//         }
//     }

//     /// Returns the address of the wallet/signer given to a client.
//     /// Matches on the [`EOA`] variant of the [`ArbiterMiddleware`] struct.
//     pub fn address(&self) -> ContractAddress {
//         self.account_address
//     }

//     /// Allows a client to set a gas price for transactions.
//     /// This can only be done if the [`Environment`] has
//     /// [`EnvironmentParameters`] `gas_settings` field set to
//     /// [`GasSettings::UserControlled`].
//     pub async fn set_gas_price(
//         &self,
//         gas_modification_request: GasModificationRequest,
//     ) -> Result<(), ArbiterCoreError> {
//         let provider = self.provider();
//         provider
//             .instruction_sender
//             .upgrade()
//             .ok_or(ArbiterCoreError::UpgradeSenderError)?
//             .send(Instruction::SetGasPrice {
//                 gas_modification_request,
//                 outcome_sender: provider.outcome_sender.clone(),
//             })?;
//         match provider.outcome_receiver.recv()?? {
//             Outcome::SetGasPriceCompleted => {
//                 debug!("Gas price set");
//                 Ok(())
//             }
//             _ => unreachable!(),
//         }
//     }

//     /// Returns a reference to the inner middleware of which there is none when
//     /// using [`ArbiterMiddleware`] so we relink to `Self`
//     fn inner(&self) -> &Connection {
//         &self.connection
//     }

//     /// Provides the default sender address for transactions, i.e., the address
//     /// of the wallet/signer given to a client of the [`Environment`].
//     fn default_sender(&self) -> Option<ContractAddress> {
//         Some(self.address())
//     }

//     /// Sends a transaction to the [`Environment`] which acts as a simulated
//     /// Ethereum network.
//     ///
//     /// The method checks if the transaction is either a call to an existing
//     /// contract or a deploy of a new one, and constructs the necessary
//     /// transaction environment used for `revm`-based transactions.
//     /// It then sends this transaction for execution and returns the
//     /// corresponding pending transaction.
//     async fn send_transaction<T: Into<BroadcastedTransaction> + Send + Sync>(
//         &self,
//         tx: T,
//         _block: Option<BlockId>,
//     ) -> Result<TransactionReceipt, ArbiterCoreError> {
//         trace!("Building transaction");

//         let instruction = Instruction::Transaction {
//             tx: tx.into(),
//             outcome_sender: self.provider().outcome_sender.clone(),
//         };

//         let provider = self.provider();
//         provider
//             .instruction_sender
//             .upgrade()
//             .ok_or(ArbiterCoreError::UpgradeSenderError)?
//             .send(instruction)?;

//         let outcome = provider.outcome_receiver.recv()??;

//         if let Outcome::TransactionCompleted(execution_result, receipt_data) = outcome {
//             match execution_result {
//                 TxExecutionResult::Success(res, tx_receipt) => Ok(tx_receipt),
//                 TxExecutionResult::Revert(reason, tx_receipt) => Ok(tx_receipt),
//             }
//         } else {
//             unreachable!()
//         }
//     }

//     /// Calls a contract method without creating a worldstate-changing
//     /// transaction on the [`Environment`] (again, simulating the Ethereum
//     /// network).
//     ///
//     /// Similar to `send_transaction`, this method checks if the call is
//     /// targeting an existing contract or deploying a new one. After
//     /// executing the call, it returns the output, but no worldstate change will
//     /// be documented in the `revm` DB.
//     async fn call(
//         &self,
//         call: FunctionCall,
//         _block: Option<BlockId>,
//     ) -> Result<Vec<Felt>, ArbiterCoreError> {
//         trace!("Building call");

//         let instruction = Instruction::Call {
//             call,
//             outcome_sender: self.provider().outcome_sender.clone(),
//         };
//         self.provider()
//             .instruction_sender
//             .upgrade()
//             .ok_or(ArbiterCoreError::UpgradeSenderError)?
//             .send(instruction)?;

//         let outcome = self.provider().outcome_receiver.recv()??;

//         if let Outcome::CallCompleted(execution_result) = outcome {
//             match execution_result {
//                 CallExecutionResult::Success(felts) => Ok(felts),
//                 CallExecutionResult::Failure(_) => Err(ArbiterCoreError::CallError {}), // TODO: add data like reason and logs
//             }
//         } else {
//             unreachable!()
//         }
//     }

//     async fn get_gas_price(&self) -> Result<u128, ArbiterCoreError> {
//         let provider = self.provider();
//         provider
//             .instruction_sender
//             .upgrade()
//             .ok_or(ArbiterCoreError::UpgradeSenderError)?
//             .send(Instruction::Query {
//                 environment_data: EnvironmentData::GasPrice,
//                 outcome_sender: provider.outcome_sender.clone(),
//             })?;

//         match provider.outcome_receiver.recv()?? {
//             Outcome::QueryReturn(outcome) => {
//                 let bytes = outcome
//                     .as_slice()
//                     .try_into()
//                     .map_err(|_| ArbiterCoreError::InvalidQueryError)?; // TODO: Might actually make sense to preserve error
//                 Ok(u128::from_be_bytes(bytes))
//             }
//             _ => unreachable!(),
//         }
//     }

//     async fn get_block_number(&self) -> Result<u64, ArbiterCoreError> {
//         let provider = self.provider();
//         provider
//             .instruction_sender
//             .upgrade()
//             .ok_or(ArbiterCoreError::UpgradeSenderError)?
//             .send(Instruction::Query {
//                 environment_data: EnvironmentData::BlockNumber,
//                 outcome_sender: provider.outcome_sender.clone(),
//             })?;

//         match provider.outcome_receiver.recv()?? {
//             Outcome::QueryReturn(outcome) => {
//                 let bytes = outcome
//                     .as_slice()
//                     .try_into()
//                     .map_err(|_| ArbiterCoreError::InvalidQueryError)?; // TODO: Might actually make sense to preserve error
//                 Ok(u64::from_be_bytes(bytes))
//             }
//             _ => unreachable!(),
//         }
//     }

//     async fn get_balance<T: Into<ContractAddress> + Send + Sync>(
//         &self,
//         from: T,
//         block: Option<BlockId>,
//     ) -> Result<BigUint, ArbiterCoreError> {
//         if block.is_some() {
//             return Err(ArbiterCoreError::InvalidQueryError);
//         }

//         let provider = self.provider();
//         provider
//             .instruction_sender
//             .upgrade()
//             .ok_or(ArbiterCoreError::UpgradeSenderError)?
//             .send(Instruction::Query {
//                 environment_data: EnvironmentData::Balance(from.into()),
//                 outcome_sender: provider.outcome_sender.clone(),
//             })?;

//         match provider.outcome_receiver.recv()?? {
//             Outcome::QueryReturn(outcome) => {
//                 let bytes = outcome
//                     .as_slice()
//                     .try_into()
//                     .map_err(|_| ArbiterCoreError::InvalidQueryError)?; // TODO: Might actually make sense to preserve error
//                 Ok(BigUint::from_bytes_be(bytes))
//             }
//             _ => unreachable!(),
//         }
//     }

//     /// Returns the timestamp of the current block.
//     pub async fn get_block_timestamp(&self) -> Result<u64, ArbiterCoreError> {
//         let provider = self.provider();
//         provider
//             .instruction_sender
//             .upgrade()
//             .ok_or(ArbiterCoreError::UpgradeSenderError)?
//             .send(Instruction::Query {
//                 environment_data: EnvironmentData::BlockTimestamp,
//                 outcome_sender: provider.outcome_sender.clone(),
//             })?;

//         match provider.outcome_receiver.recv()?? {
//             Outcome::QueryReturn(outcome) => {
//                 let bytes = outcome
//                     .as_slice()
//                     .try_into()
//                     .map_err(|_| ArbiterCoreError::InvalidQueryError)?; // TODO: Might actually make sense to preserve error
//                 Ok(u64::from_be_bytes(bytes))
//             }
//             _ => unreachable!(),
//         }
//     }

//     /// Returns the nonce of the address
//     async fn get_nonce<T: Into<ContractAddress> + Send + Sync>(
//         &self,
//         from: T,
//         _block: Option<BlockId>,
//     ) -> Result<Felt, ArbiterCoreError> {
//         let provider = self.provider();
//         provider
//             .instruction_sender
//             .upgrade()
//             .ok_or(ArbiterCoreError::UpgradeSenderError)?
//             .send(Instruction::Query {
//                 environment_data: EnvironmentData::Nonce(from.into()),
//                 outcome_sender: provider.outcome_sender.clone(),
//             })?;

//         match provider.outcome_receiver.recv()?? {
//             Outcome::QueryReturn(outcome) => {
//                 let bytes = outcome
//                     .as_slice()
//                     .try_into()
//                     .map_err(|_| ArbiterCoreError::InvalidQueryError)?; // TODO: Might actually make sense to preserve error
//                 Ok(Felt::from_bytes_be(bytes))
//             }
//             _ => unreachable!(),
//         }
//     }

//     /// Fill necessary details of a transaction for dispatch
//     ///
//     /// This function is defined on providers to behave as follows:
//     /// 1. populate the `from` field with the client address
//     /// 2. Estimate gas usage
//     ///
//     /// It does NOT set the nonce by default.

//     async fn fill_transaction(
//         &self,
//         tx: &mut BroadcastedTransaction,
//         _block: Option<BlockId>,
//     ) -> Result<(), ArbiterCoreError> {
//         // Do nothing for now

//         Ok(())
//     }
//     /// Fetches the value stored at the storage slot `key` for an account at
//     /// `address`. todo: implement the storage at a specific block feature.
//     async fn get_storage_at<T: Into<ContractAddress> + Send + Sync>(
//         &self,
//         account: T,
//         key: StorageKey,
//         block: BlockId,
//     ) -> Result<Felt, ArbiterCoreError> {
//         let result = self
//             .apply_cheatcode(Cheatcodes::Load {
//                 address: account.into(),
//                 key,
//                 block,
//             })
//             .await
//             .unwrap();

//         match result {
//             CheatcodesReturn::Load { value } => Ok(value),
//             _ => unreachable!(),
//         }
//     }
// }

#[async_trait::async_trait]
impl Provider for ArbiterMiddleware {
    async fn spec_version(&self) -> Result<String, ProviderError> {
        self.connection.spec_version().await
    }

    async fn get_block_with_tx_hashes<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithTxHashes, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection.get_block_with_tx_hashes(block_id).await
    }

    async fn get_block_with_txs<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithTxs, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection.get_block_with_txs(block_id).await
    }

    async fn get_block_with_receipts<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithReceipts, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection.get_block_with_receipts(block_id).await
    }

    async fn get_state_update<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingStateUpdate, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection.get_state_update(block_id).await
    }

    async fn get_storage_at<A, K, B>(
        &self,
        contract_address: A,
        key: K,
        block_id: B,
    ) -> Result<Felt, ProviderError>
    where
        A: AsRef<Felt> + Send + Sync,
        K: AsRef<Felt> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection
            .get_storage_at(contract_address, key, block_id)
            .await
    }

    async fn get_messages_status(
        &self,
        transaction_hash: core_types::Hash256,
    ) -> Result<Vec<core_types::MessageWithStatus>, ProviderError> {
        self.connection.get_messages_status(transaction_hash).await
    }

    async fn get_transaction_status<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionStatus, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.connection
            .get_transaction_status(transaction_hash)
            .await
    }

    async fn get_transaction_by_hash<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::Transaction, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.connection
            .get_transaction_by_hash(transaction_hash)
            .await
    }

    async fn get_transaction_by_block_id_and_index<B>(
        &self,
        block_id: B,
        index: u64,
    ) -> Result<core_types::Transaction, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection
            .get_transaction_by_block_id_and_index(block_id, index)
            .await
    }

    async fn get_transaction_receipt<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionReceiptWithBlockInfo, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.connection
            .get_transaction_receipt(transaction_hash)
            .await
    }

    async fn get_class<B, H>(
        &self,
        block_id: B,
        class_hash: H,
    ) -> Result<core_types::ContractClass, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        H: AsRef<Felt> + Send + Sync,
    {
        self.connection.get_class(block_id, class_hash).await
    }

    async fn get_class_hash_at<B, A>(
        &self,
        block_id: B,
        contract_address: A,
    ) -> Result<Felt, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        self.connection
            .get_class_hash_at(block_id, contract_address)
            .await
    }

    async fn get_class_at<B, A>(
        &self,
        block_id: B,
        contract_address: A,
    ) -> Result<core_types::ContractClass, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        self.connection
            .get_class_at(block_id, contract_address)
            .await
    }

    async fn get_block_transaction_count<B>(&self, block_id: B) -> Result<u64, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection.get_block_transaction_count(block_id).await
    }

    async fn call<R, B>(&self, request: R, block_id: B) -> Result<Vec<Felt>, ProviderError>
    where
        R: AsRef<core_types::FunctionCall> + Send + Sync,
        B: AsRef<core_types::BlockId> + Send + Sync,
    {
        self.connection.call(request, block_id).await
    }

    async fn estimate_fee<R, S, B>(
        &self,
        request: R,
        simulation_flags: S,
        block_id: B,
    ) -> Result<Vec<core_types::FeeEstimate>, ProviderError>
    where
        R: AsRef<[core_types::BroadcastedTransaction]> + Send + Sync,
        S: AsRef<[core_types::SimulationFlagForEstimateFee]> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection
            .estimate_fee(request, simulation_flags, block_id)
            .await
    }

    async fn estimate_message_fee<M, B>(
        &self,
        message: M,
        block_id: B,
    ) -> Result<core_types::FeeEstimate, ProviderError>
    where
        M: AsRef<core_types::MsgFromL1> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection
            .estimate_message_fee(message, block_id)
            .await
    }

    async fn block_number(&self) -> Result<u64, ProviderError> {
        self.connection.block_number().await
    }

    async fn block_hash_and_number(&self) -> Result<core_types::BlockHashAndNumber, ProviderError> {
        self.connection.block_hash_and_number().await
    }

    async fn chain_id(&self) -> Result<Felt, ProviderError> {
        self.connection.chain_id().await
    }

    async fn syncing(&self) -> Result<core_types::SyncStatusType, ProviderError> {
        self.connection.syncing().await
    }

    async fn get_events(
        &self,
        filter: EventFilter,
        continuation_token: Option<String>,
        chunk_size: u64,
    ) -> Result<core_types::EventsPage, ProviderError> {
        self.connection
            .get_events(filter, continuation_token, chunk_size)
            .await
    }

    async fn get_nonce<B, A>(&self, block_id: B, contract_address: A) -> Result<Felt, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        self.connection.get_nonce(block_id, contract_address).await
    }

    async fn get_storage_proof<B, H, A, K>(
        &self,
        block_id: B,
        class_hashes: H,
        contract_addresses: A,
        contracts_storage_keys: K,
    ) -> Result<core_types::StorageProof, ProviderError>
    where
        B: AsRef<core_types::ConfirmedBlockId> + Send + Sync,
        H: AsRef<[Felt]> + Send + Sync,
        A: AsRef<[Felt]> + Send + Sync,
        K: AsRef<[core_types::ContractStorageKeys]> + Send + Sync,
    {
        self.connection
            .get_storage_proof(
                block_id,
                class_hashes,
                contract_addresses,
                contracts_storage_keys,
            )
            .await
    }

    async fn add_invoke_transaction<I>(
        &self,
        invoke_transaction: I,
    ) -> Result<core_types::InvokeTransactionResult, ProviderError>
    where
        I: AsRef<core_types::BroadcastedInvokeTransaction> + Send + Sync,
    {
        self.connection
            .add_invoke_transaction(invoke_transaction)
            .await
    }

    async fn add_declare_transaction<D>(
        &self,
        declare_transaction: D,
    ) -> Result<core_types::DeclareTransactionResult, ProviderError>
    where
        D: AsRef<core_types::BroadcastedDeclareTransaction> + Send + Sync,
    {
        self.connection
            .add_declare_transaction(declare_transaction)
            .await
    }

    async fn add_deploy_account_transaction<D>(
        &self,
        deploy_account_transaction: D,
    ) -> Result<core_types::DeployAccountTransactionResult, ProviderError>
    where
        D: AsRef<core_types::BroadcastedDeployAccountTransaction> + Send + Sync,
    {
        self.connection
            .add_deploy_account_transaction(deploy_account_transaction)
            .await
    }

    async fn trace_transaction<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionTrace, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.connection.trace_transaction(transaction_hash).await
    }

    async fn simulate_transactions<B, T, S>(
        &self,
        block_id: B,
        transactions: T,
        simulation_flags: S,
    ) -> Result<Vec<core_types::SimulatedTransaction>, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        T: AsRef<[core_types::BroadcastedTransaction]> + Send + Sync,
        S: AsRef<[core_types::SimulationFlag]> + Send + Sync,
    {
        self.connection
            .simulate_transactions(block_id, transactions, simulation_flags)
            .await
    }

    async fn trace_block_transactions<B>(
        &self,
        block_id: B,
    ) -> Result<Vec<core_types::TransactionTraceWithHash>, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection.trace_block_transactions(block_id).await
    }

    async fn batch_requests<R>(
        &self,
        requests: R,
    ) -> Result<Vec<ProviderResponseData>, ProviderError>
    where
        R: AsRef<[ProviderRequestData]> + Send + Sync,
        R: 'async_trait,
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        self.connection.batch_requests(requests).await
    }
}
