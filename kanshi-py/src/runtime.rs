use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::sync::GILOnceCell;
use tokio::runtime::Runtime;

static RUNTIME: GILOnceCell<Runtime> = GILOnceCell::new();
static PID: GILOnceCell<u32> = GILOnceCell::new();

pub fn get_runtime(py: Python<'_>) -> PyResult<&'static Runtime> {
    let pid = std::process::id();
    let runtime_pid = *PID.get_or_init(py, || pid);
    if pid != runtime_pid {
        panic!(
            "Forked process detected - current PID is {} but the tokio runtime was created by {}. The tokio \
            runtime does not support forked processes https://github.com/tokio-rs/tokio/issues/4301. If you are \
            seeing this message while using Python multithreading make sure to use the `spawn` or `forkserver` \
            mode.",
            pid, runtime_pid
        );
    }

    let runtime = RUNTIME.get_or_try_init(py, || {
        Runtime::new().map_err(|err| {
            PyValueError::new_err(format!("Could not create tokio runtime. {}", err))
        })
    })?;
    Ok(runtime)
}