use pyo3::{create_exception, exceptions::PyException, prelude::*};

mod environment;
mod middleware;

/// A Python module implemented in Rust.
#[pymodule]
fn python_bindings(py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(environment::create_environment, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::create_middleware, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::declare_contract, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::create_account, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::account_execute, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::top_up_balance, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::set_storage, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::get_storage, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::call, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::impersonate, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::stop_impersonate, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::create_subscription, m)?)?;
    m.add_function(wrap_pyfunction!(middleware::poll_subscription, m)?)?;

    m.add_class::<environment::ForkParams>()?;
    Ok(())
}

create_exception!(python_bindings, ProviderError, PyException);
