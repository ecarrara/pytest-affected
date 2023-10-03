use hasher::Murmur3Hasher;
use pyo3::{pymodule, types::PyModule, PyResult, Python};
use tracer::Tracer;

mod hasher;
mod tracer;

#[pymodule]
#[pyo3(name = "_lib")]
fn pytest_affected(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<Tracer>()?;
    m.add_class::<Murmur3Hasher>()?;
    Ok(())
}
