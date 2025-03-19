mod runtime;

use futures::StreamExt;
use kanshi::{
    FileSystemEventType, FileSystemTargetKind, Kanshi, KanshiEngines, KanshiImpl, KanshiOptions,
};
use pyo3::{
    exceptions::{PyAttributeError, PyIOError, PyRuntimeError},
    prelude::*,
};
use runtime::get_runtime;

#[pyclass(unsendable)]
pub struct KanshiPy {
    kanshi: Kanshi,
}

#[pyclass(get_all)]
#[derive(Clone)]
pub struct KanshiEvent {
    pub event_type: String,
    pub target: Option<KanshiEventTarget>,
}

#[pyclass(get_all)]
#[derive(Clone)]
pub struct KanshiEventTarget {
    pub previous_path: Option<String>,
    pub new_path: Option<String>,
    pub path: String,
    pub kind: String,
}

#[pymethods]
impl KanshiPy {
    #[staticmethod]
    pub fn new(force_engine: &str) -> PyResult<KanshiPy> {
        let engine = if let Ok(engine) = KanshiEngines::from(force_engine) {
            Some(engine)
        } else {
            None
        };

        let kanshi = Kanshi::new(KanshiOptions {
            force_engine: engine,
        })
        .map_err(|e| PyIOError::new_err(e.to_string()))?;

        Ok(KanshiPy { kanshi })
    }

    pub fn watch<'py>(&self, dir: &str, py: Python<'py>) -> PyResult<()> {
        let runtime = get_runtime(py);

        if let Ok(rt) = runtime {
            py.allow_threads(|| {
                rt.block_on(async move {
                    self.kanshi
                        .watch(dir)
                        .await
                        .map_err(|e| PyRuntimeError::new_err(e.to_string()))
                })
            })
        } else {
            Err(PyRuntimeError::new_err(runtime.err().unwrap().to_string()))
        }
    }

    // py_callable signature: (event) -> None
    pub fn start<'py>(&self, py_callable: Py<PyAny>, py: Python<'py>) -> PyResult<()> {
        if !py_callable.bind_borrowed(py).is_callable() {
            Err(PyAttributeError::new_err(
                "A callable like a function, method or lambda was not passed to this method.",
            ))
        } else {
            let runtime = get_runtime(py);
            let kanshi = self.kanshi.clone();
            let mut stream = kanshi.get_events_stream();

            if let Ok(rt) = runtime {
                rt.spawn(async move {
                    while let Some(event) = stream.next().await {
                        let res = Python::with_gil(|py| -> PyResult<()> {
                            let mut previous_path: Option<String> = None;
                            let mut new_path: Option<String> = None;

                            let event_type_str = match &event.event_type {
                                FileSystemEventType::MovedFrom(path) => {
                                    previous_path = Some(path.to_str().unwrap().to_string());
                                    event.event_type.to_string()
                                }
                                FileSystemEventType::MovedTo(path) => {
                                    new_path = Some(path.to_str().unwrap().to_string());
                                    event.event_type.to_string()
                                }
                                x => x.to_string(),
                            };

                            let py_event = KanshiEvent {
                                event_type: event_type_str,
                                target: event.target.map(|x| KanshiEventTarget {
                                    previous_path,
                                    new_path,
                                    path: x.path.into_string().unwrap(),
                                    kind: match x.kind {
                                        FileSystemTargetKind::Directory => {
                                            String::from("directory")
                                        }
                                        FileSystemTargetKind::File => String::from("file"),
                                    },
                                }),
                            };

                            let py_event: Py<KanshiEvent> = Py::new(py, py_event)?;

                            py_callable.call1(py, (py_event,))?;

                            // Py::new(py, Kan)
                            Ok(())
                        });

                        if let Err(e) = res {
                            println!("{e}");
                        }
                    }
                });

                rt.spawn(async move {
                    let ret = kanshi.start().await;
                    if let Err(e) = ret {
                        println!("{e}");
                    }
                });

                Ok(())
            } else {
                Err(PyRuntimeError::new_err(runtime.err().unwrap().to_string()))
            }
        }
    }

    pub fn close(&self) -> PyResult<bool> {
        Ok(self.kanshi.close())
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn _kanshipy(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<KanshiPy>()?;
    m.add_class::<KanshiEvent>()?;
    m.add_class::<KanshiEventTarget>()?;
    Ok(())
}
