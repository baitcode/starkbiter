use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock},
};

use pyo3::prelude::*;

use starkbiter_core::{
    environment::Environment,
    middleware::{self, connection::Connection, traits::Middleware, StarkbiterMiddleware},
};
use starknet::core::types::Felt;
use starknet_accounts::{Account, SingleOwnerAccount};
use starknet_signers::{LocalWallet, SigningKey};

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
fn create_evironment(label: &str, chain_id: &str) -> PyResult<String> {
    let chain_id = Felt::from_hex(chain_id).unwrap();

    // Spin up a new environment with the specified chain ID
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .build();

    Ok("Spinning ".to_string())
}

#[pyfunction]
fn create_middleware(env_label: &str) -> PyResult<String> {
    let envs_lock = environments().lock().unwrap();
    let env = envs_lock.get(env_label);

    return if let Some(env) = env {
        let maybe_middleware = StarkbiterMiddleware::new(env, Some("random_id"));

        if let Err(e) = maybe_middleware {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to create middleware: {}",
                e
            )));
        }

        let middlewares_lock = middlewares();
        let mut middlewares = middlewares_lock.lock().unwrap();
        let id = "random_id";
        middlewares.insert("asd".to_string(), maybe_middleware.unwrap());
        Ok(id.to_string())
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
            "Environment '{}' not found",
            env_label
        )))
    };
}

fn create_account(middleware_id: &str, class_hash: &str) -> PyResult<String> {
    let middlewares_lock = middlewares().lock().unwrap();
    let middleware = middlewares_lock.get(middleware_id);

    if let Some(middleware) = middleware {
        let account = middleware
            .create_single_owner_account(
                Option::<SigningKey>::None,
                Felt::from_hex_unchecked(class_hash),
                100000000,
            )
            .await;

        Ok(account.address().to_string())
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
            "Middleware '{}' not found",
            middleware_id
        )))
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn python_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    Ok(())
}
