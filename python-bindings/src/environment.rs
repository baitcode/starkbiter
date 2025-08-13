#![allow(non_local_definitions)]
use std::{collections::HashMap, sync::OnceLock};

use pyo3::prelude::*;
use starkbiter_core::{
    environment::{self, sqlite_state_reader::SQLiteStateReader},
    tokens::{self, TokenId},
};
use starknet::providers::Url;
use starknet_core::types::Felt;
use tokio::sync::Mutex;

static ENVIRONMENTS: OnceLock<Mutex<HashMap<String, environment::Environment>>> = OnceLock::new();
pub fn env_registry() -> &'static Mutex<HashMap<String, environment::Environment>> {
    ENVIRONMENTS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[pyclass]
#[derive(FromPyObject, Debug)]
pub struct ForkParams {
    #[pyo3(get, set)]
    pub url: String,
    #[pyo3(get, set)]
    pub block: u64,
    #[pyo3(get, set)]
    pub block_hash: String,
}

#[pymethods]
impl ForkParams {
    #[new]
    fn new(url: &str, block: u64, block_hash: &str) -> Self {
        ForkParams {
            url: url.to_string(),
            block,
            block_hash: block_hash.to_string(),
        }
    }
}

#[pyfunction]
pub fn set_tracing(levels: &str) {
    std::env::set_var("RUST_LOG", levels);
    let _ = tracing_subscriber::fmt::try_init();
}

#[pyclass]
#[derive(FromPyObject)]
pub struct BridgedToken {
    /// The unique identifier for the token.
    #[pyo3(get, set)]
    pub id: String,

    /// The name of the token.
    #[pyo3(get, set)]
    pub name: String,

    /// The symbol of the token.
    #[pyo3(get, set)]
    pub symbol: String,

    /// The number of decimal places the token can be divided into.
    #[pyo3(get, set)]
    pub decimals: u8,

    /// The address of the token on the L1 (Layer 1) network.
    #[pyo3(get, set)]
    pub l1_token_address: String,

    /// The address of the bridge contract on the L1 network.
    #[pyo3(get, set)]
    pub l1_bridge_address: String,

    /// The address of the bridge contract on the L2 (Layer 2) network.
    #[pyo3(get, set)]
    pub l2_bridge_address: String,

    /// The address of the token on the L2 network.
    #[pyo3(get, set)]
    pub l2_token_address: String,
}

#[pymethods]
impl BridgedToken {
    #[allow(clippy::too_many_arguments)]
    #[new]
    fn new(
        id: String,
        name: String,
        symbol: String,
        decimals: u8,
        l1_token_address: String,
        l1_bridge_address: String,
        l2_token_address: String,
        l2_bridge_address: String,
    ) -> Self {
        BridgedToken {
            id,
            name,
            symbol,
            decimals,
            l1_token_address,
            l1_bridge_address,
            l2_token_address,
            l2_bridge_address,
        }
    }
}

impl From<tokens::BridgedToken> for BridgedToken {
    fn from(value: tokens::BridgedToken) -> Self {
        BridgedToken {
            id: value.id,
            name: value.name,
            symbol: value.symbol,
            decimals: value.decimals,
            l1_token_address: value.l1_token_address.to_hex_string(),
            l1_bridge_address: value.l1_bridge_address.to_hex_string(),
            l2_bridge_address: value.l2_bridge_address.to_hex_string(),
            l2_token_address: value.l2_token_address.to_hex_string(),
        }
    }
}

#[pyfunction]
pub fn create_environment<'p>(
    py: Python<'p>,
    label: &str,
    chain_id: &str,
    fork: Option<ForkParams>,
) -> PyResult<&'p PyAny> {
    let chain_id_local = chain_id.to_string();
    let label_local = label.to_string();

    pyo3_asyncio::tokio::future_into_py::<_, _>(py, async move {
        let chain_id = Felt::from_hex(&chain_id_local).unwrap();

        starknet_devnet_core::starknet::defaulter::StarknetDefaulter::register_defaulter(
            "sqlite",
            SQLiteStateReader::new_sqlite_state_reader,
        )
        .expect("Can't register SQLite state reader");

        // Spin up a new environment with the specified chain ID
        let mut builder = environment::Environment::builder()
            .with_chain_id(chain_id.into())
            .with_label(&label_local);

        if let Some(fork) = fork {
            tracing::info!("Forking configuration: {:?}", fork);

            let url = Url::parse(&fork.url).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid URL provided for fork: {}",
                    e
                ))
            })?;

            let block_hash = Felt::from_hex(&fork.block_hash).map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid block hash provided for fork: {}",
                    e
                ))
            })?;

            builder = builder.with_fork(url, fork.block, block_hash)
        }

        let mut envs_lock = env_registry().lock().await;
        envs_lock.insert(label_local.clone(), builder.build());

        Ok(label_local)
    })
}

#[pyfunction]
pub fn get_token<'p>(py: Python<'p>, label: &str, token: &str) -> PyResult<&'p PyAny> {
    let token_local = token.to_string();
    let label_local = label.to_string();

    pyo3_asyncio::tokio::future_into_py::<_, _>(py, async move {
        let envs_lock = env_registry().lock().await;
        let maybe_env = envs_lock.get(&label_local);

        if maybe_env.is_none() {
            return Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Environment not found for: {:?}",
                label_local
            )));
        }

        let env = maybe_env.unwrap();

        let token = TokenId::try_from(token_local.as_str()).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid token ID provided: {}",
                e
            ))
        })?;

        let maybe_chain_id = env.parameters.chain_id;
        if maybe_chain_id.is_none() {
            return Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Chain id could not be determined for: {:?}",
                label_local
            )));
        }

        let token_data = tokens::get_token_data(&maybe_chain_id.unwrap(), &token);
        if let Err(e) = token_data {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid token ID provided: {}",
                e
            )));
        }

        Ok(BridgedToken::from(token_data.unwrap()))
    })
}
