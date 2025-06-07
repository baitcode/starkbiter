use async_trait::async_trait;
use auto_impl::auto_impl;

use starknet::{providers::ProviderError, signers::VerifyingKey};
use starknet_core::types::Felt;
use starknet_devnet_types::num_bigint::BigUint;

use crate::tokens::TokenId;

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[auto_impl(&, Box, Arc)]
pub trait CheatingProvider {
    async fn create_block(&self) -> Result<(), ProviderError>;

    async fn create_account<V, F, I>(
        &self,
        public_key: V,
        class_hash: F,
        prefunded_balance: I,
    ) -> Result<Felt, ProviderError>
    where
        V: Into<VerifyingKey> + Send + Sync,
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
}
