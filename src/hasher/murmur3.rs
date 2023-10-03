use std::{
    collections::{hash_map::Entry, HashMap},
    path::PathBuf,
};

use pyo3::{pyclass, pymethods, PyResult};

#[pyclass]
pub struct Murmur3Hasher {
    cache: HashMap<PathBuf, u32>,
}

#[pymethods]
impl Murmur3Hasher {
    #[new]
    fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    fn hash_file(&mut self, filepath: PathBuf) -> PyResult<u32> {
        let hash = match self.cache.entry(filepath.clone()) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let contents = std::fs::read(filepath)?;
                let hash = murmurhash32::murmurhash3(&contents);
                *entry.insert(hash)
            }
        };

        Ok(hash)
    }
}
