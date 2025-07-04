use pyo3::{create_exception, exceptions::PyException, prelude::*};
use starknet_devnet_core::constants;

mod environment;
mod middleware;

/// A Python module implemented in Rust.
#[pymodule]
fn python_bindings(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(environment::create_environment, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::create_middleware, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::declare_contract, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::create_account, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::account_execute, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::top_up_balance, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::set_gas_price, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::set_storage, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::get_storage, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::call, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::impersonate, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::stop_impersonate, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::create_subscription, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::poll_subscription, m)?)?;

    m.add_function(wrap_pyfunction!(environment::set_tracing, m)?)?;

    m.add_class::<environment::ForkParams>()?;
    m.add_class::<middleware::BlockId>()?;
    m.add_class::<middleware::Call>()?;

    let contracts = PyModule::new(m.py(), "contracts")?;
    m.add_submodule(contracts)?;

    contracts.add(
        "UDC_CONTRACT_ADDRESS",
        constants::UDC_CONTRACT_ADDRESS.to_hex_string(),
    )?;

    contracts.add(
        "ARGENT_V040_SIERRA",
        starkbiter_bindings::ARGENT_V040_SIERRA,
    )?;
    contracts.add(
        "ERC20_CONTRACT_SIERRA",
        starkbiter_bindings::ERC20_CONTRACT_SIERRA,
    )?;
    contracts.add(
        "COUNTER_CONTRACT_SIERRA",
        starkbiter_bindings::COUNTER_CONTRACT_SIERRA,
    )?;
    contracts.add(
        "SWAPPER_CONTRACT_SIERRA",
        starkbiter_bindings::SWAPPER_CONTRACT_SIERRA,
    )?;
    contracts.add(
        "EKUBO_CORE_CONTRACT_SIERRA",
        starkbiter_bindings::EKUBO_CORE_CONTRACT_SIERRA,
    )?;
    contracts.add(
        "EKUBO_ROUTER_LITE_CONTRACT_SIERRA",
        starkbiter_bindings::EKUBO_ROUTER_LITE_CONTRACT_SIERRA,
    )?;

    Ok(())
}

create_exception!(python_bindings, ProviderError, PyException);
