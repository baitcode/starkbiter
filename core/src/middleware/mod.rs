//! The [`middleware`] module provides functionality to interact with
//! Starknet-like virtual machines. It unites Provider trait and
//! CheatingProvider trait and adds some covienience methods for working with
//! starknet-rs accounts.
//!
//! Main components:
//! - [`StarkbiterMiddleware`]: The core middleware implementation.
//! - [`Connection`]: Handles communication with the Ethereum VM.

pub mod traits;

mod cheating_provider;
use std::pin::Pin;

use async_trait;
pub use cheating_provider::CheatingProvider;
use futures::{stream, Stream, StreamExt};
use starknet::{
    core::types::{BlockId, Felt},
    providers::{Provider, ProviderError, ProviderRequestData, ProviderResponseData},
    signers::{LocalWallet, SigningKey},
};
use starknet_accounts::{ExecutionEncoding, SingleOwnerAccount};
use starknet_devnet_types::{
    num_bigint::BigUint,
    rpc::gas_modification::{GasModification, GasModificationRequest},
};

use super::*;
use crate::{
    environment::{instruction::EventFilter, Environment},
    middleware::traits::Middleware,
    tokens::TokenId,
};

pub mod connection;
use connection::*;
use starknet_core::types::{self as core_types, EmittedEvent, L1HandlerTransaction};

/// A middleware structure that integrates with `revm`.
///
/// [`StarkbiterMiddleware`] serves as a bridge between the application and
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
/// use arbiter_core::{environment::Environment, middleware::StarkbiterMiddleware};
///
/// // Create a new environment and run it
/// let mut environment = Environment::builder().build();
///
/// // Retrieve the environment to create a new middleware instance
/// let middleware = StarkbiterMiddleware::new(&environment, Some("test_label"));
/// ```
/// The client can now be used for transactions with the environment.
/// Use a seed like `Some("test_label")` for maintaining a
/// consistent address across simulations and client labeling. Seeding is be
/// useful for debugging and post-processing.
#[derive(Debug)]
pub struct StarkbiterMiddleware {
    connection: Connection,

    /// An optional label used to identify or seed the middleware instance
    /// while logging and debugging.
    #[allow(unused)]
    pub label: Option<String>,
}

impl StarkbiterMiddleware {
    /// Creates a new instance of [`StarkbiterMiddleware`].
    pub fn new(
        environment: &Environment,
        seed_and_label: Option<&str>,
    ) -> Result<Arc<Self>, Box<StarkbiterCoreError>> {
        let connection = Connection::from(environment);

        info!(
            "Created new `StarkbiterMiddleware` instance from a fork -- attached to environment labeled: {:?}",
            environment.parameters.label
        );
        Ok(Arc::new(Self {
            connection,
            label: seed_and_label.map(|s| s.to_string()),
        }))
    }

    /// Subscribes to a stream of emitted event vectors. Vectors are produced
    /// the moment new block is created. Events are filtered and only events
    /// of type `T` are returned.
    ///
    /// # Type Parameters
    /// * `T` - A type that can be constructed from a reference to
    ///   `EmittedEvent`.
    pub async fn subscribe_to<T>(&self) -> Pin<Box<dyn Stream<Item = Vec<T>> + Send + Sync>>
    where
        T: for<'a> TryFrom<&'a EmittedEvent> + Send + Sync,
    {
        return self.connection.subscribe_to().await;
    }

    /// Subscribes to a stream of flattened emitted events. similar to
    /// `subscribe_to`, but flat.
    ///
    /// # Type Parameters
    /// * `T` - A type that can be constructed from a reference to
    ///   `EmittedEvent`.
    pub async fn subscribe_to_flatten<T>(&self) -> Pin<Box<dyn Stream<Item = T> + Send + Sync>>
    where
        T: for<'a> TryFrom<&'a EmittedEvent> + Send + Sync + 'static,
    {
        let vector_stream = self.connection.subscribe_to().await;
        let item_stream = vector_stream.flat_map(stream::iter);
        Box::pin(item_stream) as Pin<Box<dyn Stream<Item = T> + Send + Sync + 'static>>
    }
}

#[async_trait::async_trait]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl Middleware for StarkbiterMiddleware {
    type Inner = Self;

    fn inner(&self) -> &Self::Inner {
        self
    }

    fn connection(&self) -> &Connection {
        &self.connection
    }

    async fn create_block(&self) -> Result<Felt, ProviderError> {
        self.connection().create_block().await
    }

    async fn get_deployed_contract_address<F>(&self, tx_hash: F) -> Result<Felt, ProviderError>
    where
        F: Into<Felt> + Send + Sync,
    {
        self.connection()
            .get_deployed_contract_address(tx_hash)
            .await
    }

    async fn create_account<V, F, I>(
        &self,
        signing_key: V,
        class_hash: F,
        prefunded_balance: I,
    ) -> Result<Felt, ProviderError>
    where
        V: Into<SigningKey> + Send + Sync, // change for Into
        F: Into<Felt> + Send + Sync,
        I: Into<BigUint> + Send + Sync,
    {
        self.connection()
            .create_account(signing_key, class_hash, prefunded_balance)
            .await
    }

    async fn create_mocked_account<F>(
        &self,
        address: F,
    ) -> Result<SingleOwnerAccount<Connection, LocalWallet>, ProviderError>
    where
        F: Into<Felt> + Send + Sync,
    {
        let signing_key: SigningKey = SigningKey::from_random();

        let chain_id = self.connection.chain_id().await?;
        let account = SingleOwnerAccount::new(
            self.connection().clone(), // TODO: not thread safe
            LocalWallet::from_signing_key(signing_key),
            address.into(),
            chain_id,
            ExecutionEncoding::New,
        );

        Ok(account)
    }

    async fn create_single_owner_account<S, F>(
        &self,
        signing_key: Option<S>,
        class_hash: F,
        prefunded_balance: u128,
    ) -> Result<SingleOwnerAccount<Connection, LocalWallet>, ProviderError>
    where
        S: Into<SigningKey> + Send + Sync,
        F: Into<Felt> + Send + Sync,
    {
        let signing_key: SigningKey = if let Some(key) = signing_key {
            key.into()
        } else {
            SigningKey::from_random()
        };

        trace!(
            "Creating single owner account with signing key: {:?}.",
            signing_key,
        );

        let address = self
            .connection()
            .create_account(signing_key.clone(), class_hash, prefunded_balance)
            .await?;

        let chain_id = self.connection.chain_id().await?;
        let account = SingleOwnerAccount::new(
            self.connection().clone(), // TODO: not thread safe
            LocalWallet::from_signing_key(signing_key),
            address,
            chain_id,
            ExecutionEncoding::New,
        );
        Ok(account)
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
        T: Into<TokenId> + Send + Sync,
    {
        self.connection()
            .top_up_balance(receiver, amount, token)
            .await
    }

    async fn get_balance<C, T>(&self, receiver: C, token: T) -> Result<BigUint, ProviderError>
    where
        C: Into<Felt> + Send + Sync,
        T: Into<TokenId> + Send + Sync,
    {
        self.connection().get_balance(receiver, token).await
    }

    async fn set_next_block_gas<G>(
        &self,
        gas_modification_request: G,
    ) -> Result<GasModification, ProviderError>
    where
        G: Into<GasModificationRequest> + Send + Sync,
    {
        self.connection()
            .set_next_block_gas(gas_modification_request)
            .await
    }

    async fn declare_contract<S>(&self, sierra_json: S) -> Result<Felt, ProviderError>
    where
        S: Into<String> + Send + Sync,
    {
        self.connection().declare_contract(sierra_json).await
    }

    async fn impersonate<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
    {
        self.connection().impersonate(address).await
    }

    async fn stop_impersonating_account<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
    {
        self.connection().stop_impersonating_account(address).await
    }

    async fn set_storage_at<C, K, V>(
        &self,
        address: C,
        key: K,
        value: V,
    ) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
        K: AsRef<Felt> + Send + Sync,
        V: AsRef<Felt> + Send + Sync,
    {
        self.connection().set_storage_at(address, key, value).await
    }

    async fn spec_version(&self) -> Result<String, ProviderError> {
        self.connection().spec_version().await
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
        self.connection().get_block_with_txs(block_id).await
    }

    async fn get_block_with_txs_from_fork<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithTxs, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection()
            .get_block_with_txs_from_fork(block_id)
            .await
    }

    async fn get_block_with_receipts<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithReceipts, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection().get_block_with_receipts(block_id).await
    }

    async fn get_state_update<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingStateUpdate, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection().get_state_update(block_id).await
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
        self.connection()
            .get_storage_at(contract_address, key, block_id)
            .await
    }

    async fn get_messages_status(
        &self,
        transaction_hash: core_types::Hash256,
    ) -> Result<Vec<core_types::MessageWithStatus>, ProviderError> {
        self.connection()
            .get_messages_status(transaction_hash)
            .await
    }

    async fn get_transaction_status<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionStatus, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.connection()
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
        self.connection()
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
        self.connection()
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
        self.connection()
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
        self.connection().get_class(block_id, class_hash).await
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
        self.connection()
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
        self.connection()
            .get_class_at(block_id, contract_address)
            .await
    }

    async fn get_block_transaction_count<B>(&self, block_id: B) -> Result<u64, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.connection()
            .get_block_transaction_count(block_id)
            .await
    }

    async fn call<R, B>(&self, request: R, block_id: B) -> Result<Vec<Felt>, ProviderError>
    where
        R: AsRef<core_types::FunctionCall> + Send + Sync,
        B: AsRef<core_types::BlockId> + Send + Sync,
    {
        self.connection().call(request, block_id).await
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
        self.connection()
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
        self.connection()
            .estimate_message_fee(message, block_id)
            .await
    }

    async fn block_number(&self) -> Result<u64, ProviderError> {
        self.connection().block_number().await
    }

    async fn block_hash_and_number(&self) -> Result<core_types::BlockHashAndNumber, ProviderError> {
        self.connection().block_hash_and_number().await
    }

    async fn chain_id(&self) -> Result<Felt, ProviderError> {
        self.connection().chain_id().await
    }

    async fn syncing(&self) -> Result<core_types::SyncStatusType, ProviderError> {
        self.connection().syncing().await
    }

    async fn get_events(
        &self,
        filter: core_types::EventFilter,
        continuation_token: Option<String>,
        chunk_size: u64,
    ) -> Result<core_types::EventsPage, ProviderError> {
        self.connection()
            .get_events(filter, continuation_token, chunk_size)
            .await
    }

    async fn get_nonce<B, A>(&self, block_id: B, contract_address: A) -> Result<Felt, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        self.connection()
            .get_nonce(block_id, contract_address)
            .await
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
        self.connection()
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
        self.connection()
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
        self.connection()
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
        self.connection()
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
        self.connection().trace_transaction(transaction_hash).await
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
        self.connection()
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
        self.connection().trace_block_transactions(block_id).await
    }

    async fn batch_requests<R>(
        &self,
        requests: R,
    ) -> Result<Vec<ProviderResponseData>, ProviderError>
    where
        R: AsRef<[ProviderRequestData]> + Send + Sync,
    {
        self.connection().batch_requests(requests).await
    }

    async fn add_l1_handler_transaction<T>(&self, tx: T) -> Result<Felt, ProviderError>
    where
        T: Into<L1HandlerTransaction> + Send + Sync,
    {
        self.connection().add_l1_handler_transaction(tx).await
    }

    async fn replay_block_with_txs<B, F>(
        &self,
        block_id: B,
        filters: F,
        override_nonce: bool,
    ) -> Result<(usize, usize, usize), ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        F: Into<Option<Vec<EventFilter>>> + Send + Sync,
    {
        self.connection()
            .replay_block_with_txs(block_id, filters, override_nonce)
            .await
    }

    async fn get_all_events(
        &self,
        from_block: Option<BlockId>,
        to_block: Option<BlockId>,
        address: Option<Felt>,
        keys: Option<Vec<Vec<Felt>>>,
    ) -> Result<Vec<core_types::EmittedEvent>, ProviderError> {
        self.connection()
            .get_all_events(from_block, to_block, address, keys)
            .await
    }
}

struct ExampleLoggingMiddleware<T: Middleware> {
    inner: T,
}

#[async_trait::async_trait]
impl<T: Middleware + Send + Sync> Middleware for ExampleLoggingMiddleware<T> {
    type Inner = T;

    fn inner(&self) -> &Self::Inner {
        &self.inner
    }

    fn connection(&self) -> &Connection {
        self.inner.connection()
    }

    async fn create_block(&self) -> Result<Felt, ProviderError> {
        trace!("Creating a block in ExampleLoggingMiddleware");
        let res = self.inner().create_block().await;
        trace!("done Creating a block in ExampleLoggingMiddleware");
        res
    }
}
