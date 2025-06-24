use pyo3::prelude::*;

use starkbiter_core::environment::Environment;
use starknet::core::types::Felt;

/// Formats the sum of two numbers as string.
#[pyfunction]
fn create_evironment(chain_id: &str) -> PyResult<String> {
    let chain_id = Felt::from_hex(chain_id).unwrap();

    // Spin up a new environment with the specified chain ID
    let env = Environment::builder()
        .with_chain_id(chain_id.into())
        .build();

    Ok("Spinning ".to_string())
}

fn create_account() -> PyResult<String> {}

/// A Python module implemented in Rust.
#[pymodule]
fn python_bindings(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sum_as_string, m)?)?;
    Ok(())
}
