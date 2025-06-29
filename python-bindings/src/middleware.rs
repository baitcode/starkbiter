use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
};

use pyo3::prelude::*;

use starkbiter_core::{
    middleware::{connection::Connection, traits::Middleware, StarkbiterMiddleware},
    tokens::TokenId,
};
use starknet_accounts::{Account, SingleOwnerAccount};
use starknet_core::types::Felt;
use starknet_signers::{LocalWallet, SigningKey};
use tokio::sync::Mutex;

use crate::environment::env_registry;

static MIDDLEWARES: OnceLock<Mutex<HashMap<String, Arc<StarkbiterMiddleware>>>> = OnceLock::new();
fn middlewares() -> &'static Mutex<HashMap<String, Arc<StarkbiterMiddleware>>> {
    MIDDLEWARES.get_or_init(|| Mutex::new(HashMap::new()))
}

static ACCOUNTS: OnceLock<Mutex<HashMap<String, SingleOwnerAccount<Connection, LocalWallet>>>> =
    OnceLock::new();
fn accounts() -> &'static Mutex<HashMap<String, SingleOwnerAccount<Connection, LocalWallet>>> {
    ACCOUNTS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[pyfunction]
pub fn create_middleware<'p>(py: Python<'p>, environment_id: &str) -> PyResult<&'p PyAny> {
    let environment_id_local = environment_id.to_string();

    pyo3_asyncio::tokio::future_into_py::<_, _>(py, async move {
        let envs_lock = env_registry().lock().await;
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

        let mut middlewares_lock = middlewares().lock().await;
        middlewares_lock.insert(random_id.to_string(), middleware);

        return Ok(random_id);
    })
}

#[pyfunction]
pub fn declare_contract<'p>(
    py: Python<'p>,
    middleware_id: &str,
    contract_json: &str,
) -> PyResult<&'p PyAny> {
    let middleware_id_local = middleware_id.to_string();
    let contract_json_local = contract_json.to_string();

    pyo3_asyncio::tokio::future_into_py::<_, _>(py, async move {
        let middlewares_lock = middlewares().lock().await;
        let maybe_middleware = middlewares_lock.get(&middleware_id_local);

        if let None = maybe_middleware {
            return Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Middleware not found for: {:?}",
                &middleware_id_local
            )));
        }

        let middleware = maybe_middleware.unwrap();

        let class_hash = middleware
            .declare_contract(contract_json_local)
            .await
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to declare contract: {}",
                    e
                ))
            })?;

        Ok(class_hash.to_string())
    })
}

#[pyfunction]
pub fn create_account<'p>(
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

        let mut accounts_lock = accounts().lock().await;

        let address = account.address().to_hex_string();

        accounts_lock.insert(address.clone(), account);

        return Ok(address);
    })
}

#[pyclass]
#[derive(FromPyObject)]
pub struct Call {
    #[pyo3(get, set)]
    pub to: String,
    #[pyo3(get, set)]
    pub selector: String,
    #[pyo3(get, set)]
    pub calldata: Vec<String>,
}

#[pymethods]
impl Call {
    #[new]
    fn new(to: &str, selector: &str, calldata: Vec<&str>) -> Self {
        Call {
            to: to.to_string(),
            selector: selector.to_string(),
            calldata: calldata.iter().map(|v| v.to_string()).collect(),
        }
    }
}

#[pyfunction]
pub fn account_execute<'p>(py: Python<'p>, address: &str, calls: Vec<Call>) -> PyResult<&'p PyAny> {
    let address_local = address.to_string();

    pyo3_asyncio::tokio::future_into_py::<_, _>(py, async move {
        let accounts_lock = accounts().lock().await;

        let maybe_account = accounts_lock.get(address_local.as_str());

        if let None = maybe_account {
            return Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Can't find account: {:?}",
                address_local
            )));
        }

        let account = maybe_account.unwrap();

        let calls = calls
            .iter()
            .map(|c| starknet_core::types::Call {
                to: Felt::from_hex_unchecked(&c.to),
                selector: Felt::from_hex_unchecked(&c.selector),
                calldata: c
                    .calldata
                    .iter()
                    .map(|v| Felt::from_hex_unchecked(v))
                    .collect(),
            })
            .collect();

        let result = account.execute_v3(calls).send().await.map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to execute account calls: {}",
                e
            ))
        })?;

        return Ok(result.transaction_hash.to_hex_string());
    })
}

#[pyfunction]
pub fn top_up_balance<'p>(
    py: Python<'p>,
    middleware_id: &str,
    address: &str,
    amount: u128,
    token: &str,
) -> PyResult<&'p PyAny> {
    let middleware_id_local = middleware_id.to_string();
    let address_local = address.to_string();
    let token_local = token.to_string();

    pyo3_asyncio::tokio::future_into_py::<_, _>(py, async move {
        let maybe_token = TokenId::try_from(token_local.as_str());
        if let Err(r) = maybe_token {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid token ID provided: {}",
                r
            )));
        }

        let middlewares_lock = middlewares().lock().await;
        let maybe_middleware = middlewares_lock.get(&middleware_id_local);

        if let None = maybe_middleware {
            return Err(PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                "Middleware not found for: {:?}",
                &middleware_id_local
            )));
        }

        let middleware = maybe_middleware.unwrap();

        middleware
            .top_up_balance(
                Felt::from_hex_unchecked(&address_local),
                amount,
                maybe_token.unwrap(),
            )
            .await
            .map_err(|e| {
                PyErr::new::<crate::ProviderException, _>(format!(
                    "Failed to top up balance: {}",
                    e
                ))
            })?;

        return Ok(());
    })
}
