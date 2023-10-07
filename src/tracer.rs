use hashbrown::HashSet;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use pyo3::{
    ffi::{_PyFrameEvalFunction, _PyInterpreterState_GetEvalFrameFunc},
    prelude::*,
    types::PyString,
    FromPyPointer,
};
use std::{ffi::c_char, os::raw::c_int};

#[pyclass]
pub struct Tracer {
    files: HashSet<String>,
    excluded_paths: Vec<String>,
    default_eval_frame_func: _PyFrameEvalFunction,
}

#[repr(C)]
pub struct _PyInterpreterFrame {
    pub f_func: *mut pyo3::ffi::PyFunctionObject,
    pub f_globals: *mut pyo3::ffi::PyObject,
    pub f_builtins: *mut pyo3::ffi::PyObject,
    pub f_locals: *mut pyo3::ffi::PyObject,
    pub f_code: *mut pyo3::ffi::PyCodeObject,
    pub frame_obj: *mut pyo3::ffi::PyFrameObject,
    pub previous: *mut _PyInterpreterFrame,
    pub prev_instr: *mut u16,
    pub stacktop: c_int,
    pub is_entry: bool,
    pub owner: c_char,
    pub localsplus: *mut pyo3::ffi::PyObject,
}

extern "C" fn eval_frame(
    state: *mut pyo3::ffi::PyThreadState,
    frame: *mut pyo3::ffi::_PyInterpreterFrame,
    throwval: c_int,
) -> *mut pyo3::ffi::PyObject {
    unsafe {
        // Temporary fix until the issue "Expose _PyInterpreterFrame_GetLine in the private API" is not resolved.
        // https://github.com/python/cpython/issues/96803
        let code = (*(frame as *mut _PyInterpreterFrame)).f_code;

        let py = Python::assume_gil_acquired();
        let filepath = PyString::from_borrowed_ptr_or_panic(py, (*code).co_filename).to_string();

        {
            let mut tracer = TRACER.write();
            tracer.add_filepath(filepath);
        }

        pyo3::ffi::_PyEval_EvalFrameDefault(state, frame, throwval)
    }
}

static TRACER: Lazy<RwLock<Tracer>> = Lazy::new(|| RwLock::new(Tracer::new()));

impl Tracer {
    fn new() -> Self {
        Python::with_gil(|py| {
            let sysconfig = py.import("sysconfig").unwrap();
            let sysconfig_get_path = sysconfig.getattr("get_path").unwrap();

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
                _PyInterpreterState_GetEvalFrameFunc(interp)
            };

            Self {
                files: HashSet::with_capacity(200),
                excluded_paths,
                default_eval_frame_func,
            }
        })
    }

    #[inline]
    fn add_filepath(&mut self, filepath: String) -> bool {
        self.files.insert(filepath)
    }
}

#[pymethods]
impl Tracer {
    #[staticmethod]
    fn start() {
        unsafe {
            let interp = pyo3::ffi::PyInterpreterState_Get();
            pyo3::ffi::_PyInterpreterState_SetEvalFrameFunc(interp, eval_frame);
        }
    }

    #[staticmethod]
    fn stop() {
        unsafe {
            let interp = pyo3::ffi::PyInterpreterState_Get();
            let tracer = TRACER.read();
            pyo3::ffi::_PyInterpreterState_SetEvalFrameFunc(interp, tracer.default_eval_frame_func);
        }
    }

    #[staticmethod]
    fn clear_files() {
        let mut tracer = TRACER.write();
        tracer.files.clear();
    }

    #[staticmethod]
    fn user_files() -> Vec<String> {
        let tracer = TRACER.read();

        tracer
            .files
            .iter()
            .filter(|path| {
                // python built-in packages
                if path.starts_with('<') {
                    return false;
                }

                if path.is_empty() {
                    return false;
                }

                for excluded_path in tracer.excluded_paths.iter() {
                    if path.starts_with(excluded_path) {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect()
    }
}
