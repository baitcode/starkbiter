use async_trait::async_trait;
use auto_impl::auto_impl;

use starknet::{
    providers::ProviderError,
    signers::{SigningKey, VerifyingKey},
};
use starknet_core::types::Felt;
use starknet_devnet_types::{
    felt::Calldata,
    num_bigint::BigUint,
    rpc::gas_modification::{GasModification, GasModificationRequest},
};

use crate::tokens::TokenId;

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[auto_impl(&, Box, Arc)]
pub trait CheatingProvider {
    async fn create_block(&self) -> Result<(), ProviderError>;

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

    async fn impersonate<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync;

    async fn stop_impersonating_account<C>(&self, address: C) -> Result<(), ProviderError>
    where
        C: AsRef<Felt> + Send + Sync;

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

    async fn declare_contract<S>(&self, sierra_json: S) -> Result<Felt, ProviderError>
    where
        S: Into<String> + Send + Sync;

    async fn set_next_block_gas<G>(
        &self,
        gas_modification_request: G,
    ) -> Result<GasModification, ProviderError>
    where
        G: Into<GasModificationRequest> + Send + Sync;

    async fn get_deployed_contract_address<F>(&self, tx_hash: F) -> Result<Felt, ProviderError>
    where
        F: Into<Felt> + Send + Sync;
}
