//! The [`traits`] module defines the [`Middleware`] trait, that allows for
//! wrapping [`Connection`] decorating it with additional functionality.
//!
//!
//! Main components:
//! - [`Middleware`]: The trait that defines the interface for middleware implementations.
//!
use async_trait;
use starknet_accounts::SingleOwnerAccount;
use starknet_devnet_types::num_bigint::BigUint;

use super::*;
use crate::tokens::TokenId;
use starknet::{
    core::types::{BlockId, Felt},
    providers::{ProviderError, ProviderRequestData, ProviderResponseData},
    signers::{LocalWallet, SigningKey},
};
use starknet_core::types::{self as core_types};

#[async_trait::async_trait]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
pub trait Middleware {
    type Inner: Middleware + Send + Sync;

    fn inner(&self) -> &Self::Inner;

    fn connection(&self) -> &Connection;

    async fn create_block(&self) -> Result<(), ProviderError> {
        self.inner().create_block().await
    }

    async fn get_deployed_contract_address<F>(&self, tx_hash: F) -> Result<Felt, ProviderError>
    where
        F: Into<Felt> + Send + Sync,
    {
        self.inner().get_deployed_contract_address(tx_hash).await
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
        self.inner()
            .create_account(signing_key, class_hash, prefunded_balance)
            .await
    }

    async fn declare_contract<S>(&self, sierra_json: S) -> Result<Felt, ProviderError>
    where
        S: Into<String> + Send + Sync,
    {
        self.inner().declare_contract(sierra_json).await
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
        self.inner()
            .create_single_owner_account(signing_key, class_hash, prefunded_balance)
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
        T: Into<TokenId> + Send + Sync,
    {
        self.inner().top_up_balance(receiver, amount, token).await
    }

    async fn impersonate<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
    {
        self.inner().impersonate(address).await
    }

    async fn stop_impersonating_account<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync,
    {
        self.inner().stop_impersonating_account(address).await
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
        self.inner().set_storage_at(address, key, value).await
    }

    async fn spec_version(&self) -> Result<String, ProviderError> {
        self.inner().spec_version().await
    }

    async fn get_block_with_tx_hashes<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithTxHashes, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().get_block_with_tx_hashes(block_id).await
    }

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

    async fn get_block_with_txs<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithTxs, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().get_block_with_txs(block_id).await
    }

    async fn get_block_with_receipts<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingBlockWithReceipts, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().get_block_with_receipts(block_id).await
    }

    async fn get_state_update<B>(
        &self,
        block_id: B,
    ) -> Result<core_types::MaybePendingStateUpdate, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().get_state_update(block_id).await
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
        self.inner()
            .get_storage_at(contract_address, key, block_id)
            .await
    }

    async fn get_messages_status(
        &self,
        transaction_hash: core_types::Hash256,
    ) -> Result<Vec<core_types::MessageWithStatus>, ProviderError> {
        self.inner().get_messages_status(transaction_hash).await
    }

    async fn get_transaction_status<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionStatus, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.inner().get_transaction_status(transaction_hash).await
    }

    async fn get_transaction_by_hash<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::Transaction, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.inner().get_transaction_by_hash(transaction_hash).await
    }

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

    async fn get_transaction_receipt<H>(
        &self,
        transaction_hash: H,
    ) -> Result<core_types::TransactionReceiptWithBlockInfo, ProviderError>
    where
        H: AsRef<Felt> + Send + Sync,
    {
        self.inner().get_transaction_receipt(transaction_hash).await
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
        self.inner().get_class(block_id, class_hash).await
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
        self.inner()
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
        self.inner().get_class_at(block_id, contract_address).await
    }

    async fn get_block_transaction_count<B>(&self, block_id: B) -> Result<u64, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
    {
        self.inner().get_block_transaction_count(block_id).await
    }

    async fn call<R, B>(&self, request: R, block_id: B) -> Result<Vec<Felt>, ProviderError>
    where
        R: AsRef<core_types::FunctionCall> + Send + Sync,
        B: AsRef<core_types::BlockId> + Send + Sync,
    {
        self.inner().call(request, block_id).await
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
        self.inner()
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
        self.inner().estimate_message_fee(message, block_id).await
    }

    async fn block_number(&self) -> Result<u64, ProviderError> {
        self.inner().block_number().await
    }

    async fn block_hash_and_number(&self) -> Result<core_types::BlockHashAndNumber, ProviderError> {
        self.inner().block_hash_and_number().await
    }

    async fn chain_id(&self) -> Result<Felt, ProviderError> {
        self.inner().chain_id().await
    }

    async fn syncing(&self) -> Result<core_types::SyncStatusType, ProviderError> {
        self.inner().syncing().await
    }

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

    async fn get_nonce<B, A>(&self, block_id: B, contract_address: A) -> Result<Felt, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync,
        A: AsRef<Felt> + Send + Sync,
    {
        self.inner().get_nonce(block_id, contract_address).await
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
        self.inner()
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
        self.inner()
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
        self.inner()
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
        self.inner()
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
        self.inner().trace_transaction(transaction_hash).await
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
        self.inner()
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
        self.inner().trace_block_transactions(block_id).await
    }

    async fn batch_requests<R>(
        &self,
        requests: R,
    ) -> Result<Vec<ProviderResponseData>, ProviderError>
    where
        R: AsRef<[ProviderRequestData]> + Send + Sync,
    {
        self.inner().batch_requests(requests).await
    }
}
