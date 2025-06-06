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
use std::sync::Mutex;

mod cheating_provider;
pub use cheating_provider::CheatingProvider;

use async_trait;
use starknet_devnet_types::{num_bigint::BigUint, starknet_api::core::ContractAddress};

use super::*;
use crate::environment::{instruction::*, Broadcast, Environment};
use starknet::{
    core::types::{BlockId, Felt},
    providers::{Provider, ProviderError, ProviderRequestData, ProviderResponseData},
    signers::VerifyingKey,
};
use starknet_core::types::{self as core_types, EventFilter};

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
/// let middleware = ArbiterMiddleware::new(&environment, Some("test_label"));
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
    /// Creates a new instance of [`ArbiterMiddleware`].
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

    /// This is a method on of the Cheatcodes that allows the to create a new
    /// user account and prefund it.
    pub async fn create_account(
        &self,
        public_key: VerifyingKey,
        class_hash: Felt,
        prefunded_balance: u128,
    ) -> Result<Felt, ProviderError> {
        return Ok(self
            .connection
            .create_account(public_key, class_hash, prefunded_balance)
            .await?);
    }

    /// Creates a new block in the environment effectively committing all
    /// pending transactions and state changes.
    pub async fn create_block(&self) -> Result<(), ProviderError> {
        return Ok(self.connection.create_block().await?);
    }
}

#[async_trait::async_trait]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl CheatingProvider for ArbiterMiddleware {
    async fn create_block(&self) -> Result<(), ProviderError> {
        self.connection.create_block().await
    }

    async fn create_account<V, F, I>(
        &self,
        public_key: V,
        class_hash: F,
        prefunded_balance: I,
    ) -> Result<Felt, ProviderError>
    where
        V: Into<VerifyingKey> + Send + Sync, // change for Into
        F: Into<Felt> + Send + Sync,
        I: Into<BigUint> + Send + Sync,
    {
        self.connection
            .create_account(public_key, class_hash, prefunded_balance)
            .await
    }

    async fn top_up_balance<C, B, T>(
        &self,
        receiver: C,
        amount: B,
        token: T,
    ) -> Result<(), ProviderError>
    where
        C: Into<Felt> + Send + Sync,
        B: Into<BigUint> + Send + Sync,
        T: Into<String> + Send + Sync,
    {
        self.connection
            .top_up_balance(receiver, amount, token)
            .await
    }

    async fn impersonate<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
    {
        self.connection.impersonate(address).await
    }

    async fn stop_impersonating_account<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
    {
        self.connection.stop_impersonating_account(address).await
    }
}

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
