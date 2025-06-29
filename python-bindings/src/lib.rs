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
    m.add_class::<environment::ForkParams>()?;
    Ok(())
}

create_exception!(python_bindings, ProviderException, PyException);
