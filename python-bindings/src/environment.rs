#![allow(non_local_definitions)]
use std::{collections::HashMap, sync::OnceLock};

use pyo3::prelude::*;
use starkbiter_core::environment;
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
pub fn set_tracing() {
    std::env::set_var("RUST_LOG", "trace");
    let _ = tracing_subscriber::fmt::try_init();
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
