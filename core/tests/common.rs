use std::sync::Arc;

use arbiter_bindings::contracts_counter::ContractsCounterReader;
use arbiter_core::{environment::Environment, middleware::ArbiterMiddleware};
use starknet::{
    macros::selector,
    providers::{Provider, ProviderError},
    signers::{LocalWallet, Signer, SigningKey},
};
use starknet_accounts::{AccountError, ConnectedAccount, ExecutionEncoding};
use starknet_core::{
    chain_id::MAINNET,
    types::{BlockId, BlockTag, Felt, FunctionCall},
    utils::get_storage_var_address,
};
use starknet_devnet_core::constants;
use starknet_devnet_types::{
    felt::join_felts, num_bigint::BigUint, starknet_api::state::StorageKey,
};

pub fn log() {
    std::env::set_var("RUST_LOG", "trace");
    tracing_subscriber::fmt::init();
}

pub fn startup() -> (Environment, Arc<ArbiterMiddleware>) {
    let env = Environment::builder().build();
    let client = ArbiterMiddleware::new(&env, Some("simple")).unwrap();
    log();
    (env, client)
}
