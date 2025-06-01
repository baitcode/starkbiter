use polars::prelude::MeltArgs;
use starknet::providers::{Provider, ProviderError};
use starknet_core::types::{self as core_types, EventFilter};
use starknet_devnet_types::rpc::block::BlockId;

trait Middleware {
    type Inner: Middleware;

    fn provider(&self) -> &Self::Provider;
}

struct Mid {}

#[async_trait::async_trait]
impl<T: Middleware> Provider for T where T: Middleware {}
