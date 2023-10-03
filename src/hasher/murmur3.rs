use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hasher,
    path::PathBuf,
};

use pyo3::{pyclass, pymethods, PyResult};

#[pyclass]
pub struct Murmur3Hasher {
    cache: HashMap<PathBuf, u64>,
}

#[pymethods]
impl Murmur3Hasher {
    #[new]
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    fn hash_file(&mut self, filepath: PathBuf) -> PyResult<u64> {
        let hash = match self.cache.entry(filepath.clone()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let contents = std::fs::read(filepath)?;
                let mut hasher = fasthash::Murmur3Hasher::default();
                hasher.write(&contents);
                let hash = hasher.finish();
                *entry.insert(hash)
            }
        };

        Ok(hash)
    }
}
