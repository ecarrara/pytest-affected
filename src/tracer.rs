use std::collections::HashSet;

use pyo3::{prelude::*, types::PyString};

#[pyclass]
pub struct Tracer {
    files: HashSet<String>,
    excluded_paths: Vec<String>,
}

#[pymethods]
impl Tracer {
    #[new]
    fn new(py: Python<'_>) -> PyResult<Self> {
        let sysconfig = py.import("sysconfig")?;
        let sysconfig_get_path = sysconfig.getattr("get_path")?;

        let excluded_paths = ["stdlib", "purelib", "platstdlib", "platlib"]
            .into_iter()
            .filter_map(|name| {
                sysconfig_get_path
                    .call1((name,))
                    .ok()
                    .map(|path| path.to_string())
            })
            .collect();

        Ok(Self {
            files: HashSet::new(),
            excluded_paths,
        })
    }

    fn tracefunc(
        mut slf: PyRefMut<'_, Self>,
        py: Python<'_>,
        frame: &PyAny,
        event: &PyString,
        _arg: &PyAny,
    ) -> PyResult<Py<PyAny>> {
        if event.compare("call")?.is_eq() {
            // Disable per-line trace events for this frame (?).
            frame.setattr("f_trace_lines", false)?;

            let co_filename = frame.getattr("f_code")?.getattr("co_filename")?;
            slf.files.insert(co_filename.to_string());
        }

        slf.into_py(py).getattr(py, "tracefunc")
    }

    #[getter]
    fn user_files(&self) -> Vec<String> {
        self.files
            .iter()
            .filter(|path| {
                // python built-in packages
                if path.starts_with("<") {
                    return false;
                }

                if path.is_empty() {
                    return false;
                }

                for excluded_path in self.excluded_paths.iter() {
                    if path.starts_with(excluded_path) {
                        return false;
                    }
                }

                return true;
            })
            .cloned()
            .collect()
    }
}
