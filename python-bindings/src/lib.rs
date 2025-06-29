use std::{
    collections::HashMap,
    ops::Bound,
    sync::{Arc, OnceLock},
};

use pyo3::prelude::*;

use starkbiter_core::{
    environment::Environment,
    middleware::{connection::Connection, traits::Middleware, StarkbiterMiddleware},
};
use starknet::core::types::Felt;
use starknet_accounts::{Account, SingleOwnerAccount}
use starknet_signers::{LocalWallet, SigningKey};
use tokio::sync::Mutex;

static ACCOUNTS: OnceLock<Mutex<HashMap<String, SingleOwnerAccount<Connection, LocalWallet>>>> =
    OnceLock::new();
fn accounts() -> &'static Mutex<HashMap<String, SingleOwnerAccount<Connection, LocalWallet>>> {
    ACCOUNTS.get_or_init(|| Mutex::new(HashMap::new()))
}

static ENVIRONMENTS: OnceLock<Mutex<HashMap<String, Environment>>> = OnceLock::new();
fn environments() -> &'static Mutex<HashMap<String, Environment>> {
    ENVIRONMENTS.get_or_init(|| Mutex::new(HashMap::new()))
}

static MIDDLEWARES: OnceLock<Mutex<HashMap<String, Arc<StarkbiterMiddleware>>>> = OnceLock::new();
fn middlewares() -> &'static Mutex<HashMap<String, Arc<StarkbiterMiddleware>>> {
    MIDDLEWARES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Formats the sum of two numbers as string.
#[pyfunction]
fn create_evironment<'p>(py: Python<'p>, label: &str, chain_id: &str) -> PyResult<&'p PyAny> {
    let chain_id_local = chain_id.to_string();
    let label_local = label.to_string();

    pyo3_asyncio::tokio::future_into_py::<_, _>(py, async move {
        let chain_id = Felt::from_hex(&chain_id_local).unwrap();

        // Spin up a new environment with the specified chain ID
        let env = Environment::builder()
            .with_chain_id(chain_id.into())
            .with_label(&label_local)
            .build();

        let mut envs_lock = environments().lock().await;
        envs_lock.insert(label_local, env);

        Ok("asd".to_string())
    })
}

#[pyfunction]
fn create_middleware<'p>(py: Python<'p>, environment_id: &str) -> PyResult<&'p PyAny> {
    let environment_id_local = environment_id.to_string();

    pyo3_asyncio::tokio::future_into_py::<_, _>(py, async move {
        let envs_lock = environments().lock().await;
        let maybe_env = envs_lock.get(&environment_id_local);

        if let None = maybe_env {
            return Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Environment not found for: {:?}",
                environment_id_local
            )));
        }

        let env = maybe_env.unwrap();

        let random_id = "random_id";
        let middleware = StarkbiterMiddleware::new(env, Some(random_id)).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create middleware: {}",
                e
            ))
        })?;

        return Ok(random_id);
    })
}

#[pyfunction]
fn create_account<'p>(
    py: Python<'p>,
    middleware_id: &str,
    class_hash: &str,
) -> PyResult<&'p PyAny> {
    let middleware_id_local = middleware_id.to_string();
    let class_hash_id_local = class_hash.to_string();

    pyo3_asyncio::tokio::future_into_py::<_, _>(py, async move {
        let middlewares_lock = middlewares().lock();
        let guard = middlewares_lock.await;
        let maybe_middleware = guard.get(middleware_id_local.as_str());

        if let None = maybe_middleware {
            return Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Middleware not found for: {:?}",
                &middleware_id_local
            )));
        }

        let middleware = maybe_middleware.unwrap();

        let maybe_account = middleware
            .create_single_owner_account(
                Option::<SigningKey>::None,
                Felt::from_hex_unchecked(&class_hash_id_local),
                100000000,
            )
            .await;

        if let Err(e) = maybe_account {
            return Err(PyErr::new::<pyo3::exceptions::PyException, _>(format!(
                "Can't create account: {:?}",
                e
            )));
        }

        let account = maybe_account.unwrap();

        return Ok(account.address().to_string());
    })
}

/// A Python module implemented in Rust.
#[pymodule]
fn python_bindings(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(create_account, m)?)?;
    m.add_function(wrap_pyfunction!(create_evironment, m)?)?;
    m.add_function(wrap_pyfunction!(create_middleware, m)?)?;
    Ok(())
}
