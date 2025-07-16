use async_trait::async_trait;
use auto_impl::auto_impl;
use starknet::{providers::ProviderError, signers::SigningKey};
use starknet_core::types::{
    BlockId, EmittedEvent, Felt, L1HandlerTransaction, MaybePendingBlockWithTxs,
};
use starknet_devnet_types::{
    num_bigint::BigUint,
    rpc::gas_modification::{GasModification, GasModificationRequest},
};

use crate::{environment::instruction::EventFilter, tokens::TokenId};

/// A trait for providing cheating functionalities such as creating accounts,
/// topping up balances, and impersonating accounts.
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[auto_impl(&, Box, Arc)]
pub trait CheatingProvider {
    /// Makes underlying provider to create a new block out of all pending
    /// changes. Returns the hash of the created block.
    async fn create_block(&self) -> Result<Felt, ProviderError>;

    /// Creates a new account with the given signing key, class hash, and
    /// prefunded balance. Returns the address of the created account.
    async fn create_account<V, F, I>(
        &self,
        signing_key: V,
        class_hash: F,
        prefunded_balance: I,
    ) -> Result<Felt, ProviderError>
    where
        V: Into<SigningKey> + Send + Sync,
        F: Into<Felt> + Send + Sync,
        I: Into<BigUint> + Send + Sync;

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
        T: Into<TokenId> + Send + Sync;

    /// Get token balance of the given address. Uses smallest denomination of
    /// the token.
    async fn get_balance<C, T>(&self, receiver: C, token: T) -> Result<BigUint, ProviderError>
    where
        C: Into<Felt> + Send + Sync,
        T: Into<TokenId> + Send + Sync;

    /// Registers address for impersonation. Means that validation step for all
    /// transactions from that address will be skipped.
    async fn impersonate<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync;

    /// Deregisters address for impersonation.
    async fn stop_impersonating_account<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync;

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
        V: AsRef<Felt> + Send + Sync;

    /// Declares a contract with the given Sierra JSON.
    /// Returns the class hash of the declared contract.
    async fn declare_contract<S>(&self, sierra_json: S) -> Result<Felt, ProviderError>
    where
        S: Into<String> + Send + Sync;

    /// Manipulates the next block's gas settings. Can also produce a block.
    async fn set_next_block_gas<G>(
        &self,
        gas_modification_request: G,
    ) -> Result<GasModification, ProviderError>
    where
        G: Into<GasModificationRequest> + Send + Sync;

    /// Checks if transaction contains UDC deploy contract event fetches first
    /// and returns address from it.
    async fn get_deployed_contract_address<F>(&self, tx_hash: F) -> Result<Felt, ProviderError>
    where
        F: Into<Felt> + Send + Sync;

    /// Gets block information with full transactions and receipts given the
    /// block id. Looks up the underlying fork.
    async fn get_block_with_txs_from_fork<B>(
        &self,
        block_id: B,
    ) -> Result<MaybePendingBlockWithTxs, ProviderError>
    where
        B: AsRef<BlockId> + Send + Sync;

    /// Adds a L1 handler transaction to blockchain.
    async fn add_l1_handler_transaction<T>(&self, tx: T) -> Result<Felt, ProviderError>
    where
        T: Into<L1HandlerTransaction> + Send + Sync;

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
        F: Into<Option<Vec<EventFilter>>> + Send + Sync;

    /// Gets all events in one go given filter parameters.
    async fn get_all_events(
        &self,
        from_block: Option<BlockId>,
        to_block: Option<BlockId>,
        address: Option<Felt>,
        keys: Option<Vec<Vec<Felt>>>,
    ) -> Result<Vec<EmittedEvent>, ProviderError>;
}
