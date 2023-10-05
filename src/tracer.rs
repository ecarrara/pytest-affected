use hashbrown::HashSet;
use pyo3::{
    prelude::*,
    types::{PyDict, PyString},
    FromPyPointer,
};

use std::os::raw::c_int;

#[pyclass]
pub struct Tracer {
    files: HashSet<String>,
    excluded_paths: Vec<String>,
    default_eval_frame_func: pyo3::ffi::_PyFrameEvalFunction,
}

extern "C" {
    pub fn _PyEval_EvalFrameDefault(
        tstate: *mut pyo3::ffi::PyThreadState,
        frame: *mut pyo3::ffi::PyFrameObject,
        throwval: c_int,
    ) -> *mut pyo3::ffi::PyObject;

    pub fn PyThreadState_GetInterpreter(
        tstate: *mut pyo3::ffi::PyThreadState,
    ) -> *mut pyo3::ffi::PyInterpreterState;

}

extern "C" fn eval_frame(
    state: *mut pyo3::ffi::PyThreadState,
    frame: *mut pyo3::ffi::PyFrameObject,
    throwval: c_int,
) -> *mut pyo3::ffi::PyObject {
    unsafe {
        Python::with_gil_unchecked(|py| {
            let current_frame = pyo3::ffi::PyEval_GetFrame();
            if !current_frame.is_null() {
                let code = pyo3::ffi::PyFrame_GetCode(current_frame);

                let filepath = code
                    .as_ref()
                    .map(|code| PyString::from_borrowed_ptr_or_panic(py, code.co_filename))
                    .unwrap();

                let thread_state =
                    PyDict::from_borrowed_ptr_or_panic(py, pyo3::ffi::PyThreadState_GetDict());

                thread_state.get_item("_affected_tracer").map(|tracer| {
                    tracer
                        .getattr("add_filepath")
                        .map(|func| func.call1((filepath,)))
                });
            }
        });
        _PyEval_EvalFrameDefault(state, frame, throwval)
    }
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

        let default_eval_frame_func = unsafe {
            let interp = pyo3::ffi::PyInterpreterState_Get();
            pyo3::ffi::_PyInterpreterState_GetEvalFrameFunc(interp)
        };

        Ok(Self {
            files: HashSet::with_capacity(200),
            excluded_paths,
            default_eval_frame_func,
        })
    }

    fn start(slf: PyRefMut<'_, Self>, py: Python<'_>) {
        unsafe {
            let tstate = pyo3::ffi::PyThreadState_Get();
            let interp = PyThreadState_GetInterpreter(tstate);

            let thread_state = pyo3::ffi::PyThreadState_GetDict();
            let key = PyString::intern(py, "_affected_tracer");
            pyo3::ffi::PyObject_SetItem(thread_state, key.into_ptr(), slf.into_ptr());

            pyo3::ffi::_PyInterpreterState_SetEvalFrameFunc(interp, eval_frame);
        }
    }

    fn stop(&self) {
        unsafe {
            let tstate = pyo3::ffi::PyThreadState_Get();
            let interp = PyThreadState_GetInterpreter(tstate);
            pyo3::ffi::_PyInterpreterState_SetEvalFrameFunc(interp, self.default_eval_frame_func);
        }
    }

    fn add_filepath(&mut self, filepath: String) -> bool {
        self.files.insert(filepath)
    }

    fn clear_files(&mut self) {
        self.files.clear()
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
