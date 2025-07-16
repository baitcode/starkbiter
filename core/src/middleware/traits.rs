//! The [`traits`] module defines the [`Middleware`] trait, that allows for
//! wrapping [`Connection`] decorating it with additional functionality.
//!
//!
//! Main components:
//! - [`Middleware`]: The trait that defines the interface for middleware
//!   implementations.
use async_trait::async_trait;
use auto_impl::auto_impl;
use starknet::{
    core::types::{BlockId, Felt},
    providers::{ProviderError, ProviderRequestData, ProviderResponseData},
    signers::{LocalWallet, SigningKey},
};
use starknet_accounts::SingleOwnerAccount;
use starknet_core::types::{self as core_types, L1HandlerTransaction};
use starknet_devnet_types::num_bigint::BigUint;

use super::*;
use crate::tokens::TokenId;

/// The `Middleware` trait defines the interface for middleware implementations.
///
/// Middleware wraps Connection and/or other middleware, allowing for method
/// decoration.
///
/// It implements all the methods from Provider trait and CheatingProvider
/// trait, as well as additional convenience methods for dealing with accounts
/// from starknet-rs crate.
///
/// This trait contains default implementations for all methods, proxying all
/// calls to the underlying layer. Use with caution, as it may lead to stack
/// overflow error.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[auto_impl(&, Box, Arc)]
pub trait Middleware {
    /// The inner type of the middleware, which is another Middleware
    /// implementation.
    type Inner: Middleware + Send + Sync;

    /// Returns the reference to the middleware it wraps.
    fn inner(&self) -> &Self::Inner;

    /// Returns the connection to the underlying provider (the deepest
    /// middleware's connection).
    fn connection(&self) -> &Connection;

    /// Creates a new block in the underlying provider.
    async fn create_block(&self) -> Result<Felt, ProviderError> {
        self.inner().create_block().await
    }

    /// Checks if transaction contains UDC deploy contract event fetches first
    /// and returns address from it.
    async fn get_deployed_contract_address<F>(&self, tx_hash: F) -> Result<Felt, ProviderError>
    where
        F: Into<Felt> + Send + Sync,
    {
        self.inner().get_deployed_contract_address(tx_hash).await
    }

    /// Creates a new account with the given signing key, class hash, and
    /// prefunded balance. Returns the address of the created account.
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
        self.inner()
            .create_account(signing_key, class_hash, prefunded_balance)
            .await
    }

    /// Declares a contract with the given Sierra JSON.
    /// Returns the class hash of the declared contract.
    async fn declare_contract<S>(&self, sierra_json: S) -> Result<Felt, ProviderError>
    where
        S: Into<String> + Send + Sync,
    {
        self.inner().declare_contract(sierra_json).await
    }

    /// Creates a mocked single owner account without actually creating an
    /// account, such an account needed to impersonate owner.
    async fn create_mocked_account<F>(
        &self,
        address: F,
    ) -> Result<SingleOwnerAccount<Connection, LocalWallet>, ProviderError>
    where
        F: Into<Felt> + Send + Sync,
    {
        self.inner().create_mocked_account(address).await
    }

    /// Creates a single owner account with the given signing key,
    /// class hash, and prefunded balance. Needed for cainome contract bindings.
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
        self.inner()
            .create_single_owner_account(signing_key, class_hash, prefunded_balance)
            .await
    }

    /// Top up the balance of the given receiver with the specified amount and
    /// token. Uses smallest denomination of the token.
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
        self.inner().top_up_balance(receiver, amount, token).await
    }

    /// Get token balance of the given. Uses smallest denomination of the token.
    async fn get_balance<C, T>(&self, receiver: C, token: T) -> Result<BigUint, ProviderError>
    where
        C: Into<Felt> + Send + Sync,
        T: Into<TokenId> + Send + Sync,
    {
        self.inner().get_balance(receiver, token).await
    }

    /// Registers address for impersonation. Means that validation step for all
    /// transactions from that address will be skipped.
    async fn impersonate<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
    {
        self.inner().impersonate(address).await
    }

    /// Deregisters address for impersonation.
    async fn stop_impersonating_account<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
    {
        self.inner().stop_impersonating_account(address).await
    }

    /// Directly manipulates contracts storage at a given address.
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
        self.inner().set_storage_at(address, key, value).await
    }

    /// This is a part for JSON-RPC spec. Returns some value. 8 )
    async fn spec_version(&self) -> Result<String, ProviderError> {
        self.inner().spec_version().await
    }

    /// This is a part for JSON-RPC spec. Returns the cheapest block data.
    async fn get_block_with_tx_hashes<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithTxHashes, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().get_block_with_tx_hashes(block_id).await
    }

    /// Manipulates the next block's gas settings. Can also produce a block.
    async fn set_next_block_gas<G>(
        &self,
        gas_modification_request: G,
    ) -> Result<GasModification, ProviderError>
    where
        G: Into<GasModificationRequest> + Send + Sync,
    {
        self.inner()
            .set_next_block_gas(gas_modification_request)
            .await
    }

    /// Returns block with full transaction data. Looks up only internal state.
    async fn get_block_with_txs<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithTxs, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().get_block_with_txs(block_id).await
    }

    /// Returns block with full transaction data. Looks up only fork.
    async fn get_block_with_txs_from_fork<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithTxs, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().get_block_with_txs_from_fork(block_id).await
    }

    /// Returns block with transaction receipts only.
    async fn get_block_with_receipts<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithReceipts, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().get_block_with_receipts(block_id).await
    }

    /// Returns data in the pending state of the node.
    async fn get_state_update<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingStateUpdate, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().get_state_update(block_id).await
    }

    /// Return the storage value at the given contract address and key for the
    /// specified block.
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
        self.inner()
            .get_storage_at(contract_address, key, block_id)
            .await
    }

    /// Returns status of L1->L2 messages for a given transaction hash.
    async fn get_messages_status(
        &self,
        transaction_hash: core_types::Hash256,
    ) -> Result<Vec<core_types::MessageWithStatus>, ProviderError> {
        self.inner().get_messages_status(transaction_hash).await
    }

    /// Returns the status of a transaction by its hash.
    async fn get_transaction_status<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionStatus, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.inner().get_transaction_status(transaction_hash).await
    }

    /// Returns the transaction by its hash.
    async fn get_transaction_by_hash<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::Transaction, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.inner().get_transaction_by_hash(transaction_hash).await
    }

    /// Returns the transaction by block ID and trancsaction index within the
    /// block.
    async fn get_transaction_by_block_id_and_index<B>(
        &self,
        block_id: B,
        index: u64,
    ) -> Result<core_types::Transaction, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner()
            .get_transaction_by_block_id_and_index(block_id, index)
            .await
    }

    /// Returns the transaction receipt by its hash.
    async fn get_transaction_receipt<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionReceiptWithBlockInfo, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.inner().get_transaction_receipt(transaction_hash).await
    }

    /// Returns contract class data by class hash.
    async fn get_class<B, H>(
        &self,
        block_id: B,
        class_hash: H,
    ) -> Result<core_types::ContractClass, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        H: AsRef<Felt> + Send + Sync,
    {
        self.inner().get_class(block_id, class_hash).await
    }

    /// Returns the class hash at a given contract address for the specified
    /// block.
    async fn get_class_hash_at<B, A>(
        &self,
        block_id: B,
        contract_address: A,
    ) -> Result<Felt, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        self.inner()
            .get_class_hash_at(block_id, contract_address)
            .await
    }

    /// Returns the class at a given contract address for the specified block.
    async fn get_class_at<B, A>(
        &self,
        block_id: B,
        contract_address: A,
    ) -> Result<core_types::ContractClass, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        self.inner().get_class_at(block_id, contract_address).await
    }

    /// Returns the transaction count for a given block ID.
    async fn get_block_transaction_count<B>(&self, block_id: B) -> Result<u64, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().get_block_transaction_count(block_id).await
    }

    /// Executes read-only function calls against the state of the node.
    async fn call<R, B>(&self, request: R, block_id: B) -> Result<Vec<Felt>, ProviderError>
    where
        R: AsRef<core_types::FunctionCall> + Send + Sync,
        B: AsRef<core_types::BlockId> + Send + Sync,
    {
        self.inner().call(request, block_id).await
    }

    /// Simulates transactions against the state of the node and calculates the
    /// gas used.
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
        self.inner()
            .estimate_fee(request, simulation_flags, block_id)
            .await
    }

    /// ??? Not sure what this is for, but it is in the Provider trait.
    async fn estimate_message_fee<M, B>(
        &self,
        message: M,
        block_id: B,
    ) -> Result<core_types::FeeEstimate, ProviderError>
    where
        M: AsRef<core_types::MsgFromL1> + Send + Sync,
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().estimate_message_fee(message, block_id).await
    }

    /// Returns the current block number.
    async fn block_number(&self) -> Result<u64, ProviderError> {
        self.inner().block_number().await
    }

    /// Returns the current block hash and number.
    async fn block_hash_and_number(&self) -> Result<core_types::BlockHashAndNumber, ProviderError> {
        self.inner().block_hash_and_number().await
    }

    /// Returns the chain ID of the network.
    async fn chain_id(&self) -> Result<Felt, ProviderError> {
        self.inner().chain_id().await
    }

    /// Checks for state, by default it is `false` (inherited from devnet
    /// implementation).
    async fn syncing(&self) -> Result<core_types::SyncStatusType, ProviderError> {
        self.inner().syncing().await
    }

    /// Fetches events based on the provided filter. Allows for pagination using
    /// a continuation token and chunk size.
    async fn get_events(
        &self,
        filter: core_types::EventFilter,
        continuation_token: Option<String>,
        chunk_size: u64,
    ) -> Result<core_types::EventsPage, ProviderError> {
        self.inner()
            .get_events(filter, continuation_token, chunk_size)
            .await
    }

    /// Fetches all events based on the provided filter.
    async fn get_all_events(
        &self,
        from_block: Option<BlockId>,
        to_block: Option<BlockId>,
        address: Option<Felt>,
        keys: Option<Vec<Vec<Felt>>>,
    ) -> Result<Vec<core_types::EmittedEvent>, ProviderError> {
        self.inner()
            .get_all_events(from_block, to_block, address, keys)
            .await
    }

    /// Returns the nonce of a contract at a given block ID.
    async fn get_nonce<B, A>(&self, block_id: B, contract_address: A) -> Result<Felt, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        self.inner().get_nonce(block_id, contract_address).await
    }

    /// Returns the storage proof for the specified block, class hashes,
    /// contract addresses,
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
        self.inner()
            .get_storage_proof(
                block_id,
                class_hashes,
                contract_addresses,
                contracts_storage_keys,
            )
            .await
    }

    /// Sends invoke transaction for execution
    async fn add_invoke_transaction<I>(
        &self,
        invoke_transaction: I,
    ) -> Result<core_types::InvokeTransactionResult, ProviderError>
    where
        I: AsRef<core_types::BroadcastedInvokeTransaction> + Send + Sync,
    {
        self.inner()
            .add_invoke_transaction(invoke_transaction)
            .await
    }

    /// Sends declare transaction for execution. (should not be used, check
    /// declare_contract method)
    async fn add_declare_transaction<D>(
        &self,
        declare_transaction: D,
    ) -> Result<core_types::DeclareTransactionResult, ProviderError>
    where
        D: AsRef<core_types::BroadcastedDeclareTransaction> + Send + Sync,
    {
        self.inner()
            .add_declare_transaction(declare_transaction)
            .await
    }

    /// Sends deploy account transaction for execution.
    async fn add_deploy_account_transaction<D>(
        &self,
        deploy_account_transaction: D,
    ) -> Result<core_types::DeployAccountTransactionResult, ProviderError>
    where
        D: AsRef<core_types::BroadcastedDeployAccountTransaction> + Send + Sync,
    {
        self.inner()
            .add_deploy_account_transaction(deploy_account_transaction)
            .await
    }

    /// Returns the transaction trace for a given transaction hash.
    async fn trace_transaction<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionTrace, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.inner().trace_transaction(transaction_hash).await
    }

    /// Simulates transactions against the state of the node and returns the
    /// results without changing the state.
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
        self.inner()
            .simulate_transactions(block_id, transactions, simulation_flags)
            .await
    }

    /// Returns the transaction traces for a given block ID.
    async fn trace_block_transactions<B>(
        &self,
        block_id: B,
    ) -> Result<Vec<core_types::TransactionTraceWithHash>, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().trace_block_transactions(block_id).await
    }

    /// Executes batch requests. Currently, it is not implemented and will
    /// return an error.
    async fn batch_requests<R>(
        &self,
        requests: R,
    ) -> Result<Vec<ProviderResponseData>, ProviderError>
    where
        R: AsRef<[ProviderRequestData]> + Send + Sync,
    {
        self.inner().batch_requests(requests).await
    }

    /// Adds a L1 handler transaction to blockchain.
    async fn add_l1_handler_transaction<T>(&self, tx: T) -> Result<Felt, ProviderError>
    where
        T: Into<L1HandlerTransaction> + Send + Sync,
    {
        self.inner().add_l1_handler_transaction(tx).await
    }

    /// Gets block information from original forked blockchain and replays
    /// against pending block of the local version.
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
        self.inner()
            .replay_block_with_txs(block_id, filters, override_nonce)
            .await
    }
}
