//! The embedded interpreter: one Python thread for the process, the
//! native `_operandi` module, and the `operandi` module written in Python
//! (operandi.py) on top of it.
use crate::{Msg, Outcome, Printed, Reply, Request, Source};
use pyo3::buffer::PyBuffer;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::cell::RefCell;
use std::ffi::{CString, c_long};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Mutex, OnceLock};

/// The `operandi` module's Python half.
const OPERANDI_PY: &str = include_str!("operandi.py");

struct Job {
    source: Source,
    msgs: Sender<Msg>,
    replies: Receiver<Reply>,
}

struct Worker {
    jobs: Mutex<Sender<Job>>,
    /// The Python thread's id (`PyThread_get_thread_ident`), for Stop.
    ident: AtomicU64,
    busy: AtomicBool,
    version: String,
}

static WORKER: OnceLock<Result<Worker, String>> = OnceLock::new();

/// A running script's link to its host: where its calls go, and where
/// the answers come back (out while a call waits for its answer).
type Link = (Sender<Msg>, Option<Receiver<Reply>>);

thread_local! {
    /// The running script's link to its host (Python thread only).
    static CURRENT: RefCell<Option<Link>> = const { RefCell::new(None) };
    /// The console's names, kept between console lines.
    static CONSOLE: RefCell<Option<Py<PyDict>>> = const { RefCell::new(None) };
}

fn worker() -> Result<&'static Worker, String> {
    WORKER.get_or_init(spawn).as_ref().map_err(Clone::clone)
}

pub(crate) fn ensure() -> Result<(), String> {
    worker().map(|_| ())
}

pub(crate) fn version() -> Option<String> {
    worker().ok().map(|w| w.version.clone())
}

pub(crate) fn start(source: Source) -> Result<(Receiver<Msg>, Sender<Reply>), String> {
    let w = worker()?;
    if w.busy.swap(true, Ordering::SeqCst) {
        return Err(crate::BUSY.into());
    }
    let (msgs_tx, msgs_rx) = mpsc::channel();
    let (replies_tx, replies_rx) = mpsc::channel();
    let job = Job {
        source,
        msgs: msgs_tx,
        replies: replies_rx,
    };
    let sent = w.jobs.lock().unwrap_or_else(|e| e.into_inner()).send(job);
    if sent.is_err() {
        w.busy.store(false, Ordering::SeqCst);
        return Err("The Python thread stopped".into());
    }
    Ok((msgs_rx, replies_tx))
}

pub(crate) fn interrupt() {
    let Ok(w) = worker() else {
        return;
    };
    if !w.busy.load(Ordering::SeqCst) {
        return;
    }
    let ident = w.ident.load(Ordering::SeqCst) as c_long;
    Python::attach(|_py| {
        // SAFETY: the interpreter is initialised (the worker exists) and
        // this thread is attached for the call, as SetAsyncExc requires;
        // `ident` is the Python thread's own id and the exception type is
        // a borrowed static the interpreter owns.
        unsafe {
            pyo3::ffi::PyThreadState_SetAsyncExc(ident, pyo3::ffi::PyExc_KeyboardInterrupt);
        }
    });
}

/// Where Python's standard library is: `OPERANDI_PYTHON_HOME`, else the
/// app bundle's copy, else the Python this build linked against.
fn home() -> Option<String> {
    if let Ok(h) = std::env::var("OPERANDI_PYTHON_HOME") {
        return Some(h);
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        for bundled in [dir.join("../Resources/python"), dir.join("python")] {
            if bundled.join("lib").is_dir() {
                return Some(bundled.to_string_lossy().into_owned());
            }
        }
    }
    option_env!("OPERANDI_PYTHON_BUILD_HOME").map(str::to_owned)
}

/// Starts CPython isolated from the user's environment (no PYTHONPATH,
/// no user site-packages, no signal handlers) with `home`'s library.
fn initialize() -> Result<(), String> {
    use pyo3::ffi;
    pyo3::append_to_inittab!(_operandi);
    let home = home()
        .map(CString::new)
        .transpose()
        .map_err(|e| e.to_string())?;
    // SAFETY: PyConfig is a plain C struct that PyConfig_InitIsolatedConfig
    // fills in completely before anything reads it; the strings it holds
    // are copied by PyConfig_SetBytesString and freed by PyConfig_Clear,
    // which runs on every path after the init. Nothing else in the process
    // initialises Python (pyo3's auto-initialize is off) and this runs
    // once, on the thread that then owns the interpreter.
    unsafe {
        let mut config = std::mem::MaybeUninit::<ffi::PyConfig>::zeroed().assume_init();
        ffi::PyConfig_InitIsolatedConfig(&mut config);
        let status_text = |s: ffi::PyStatus| {
            if s.err_msg.is_null() {
                "Python failed to start".to_owned()
            } else {
                std::ffi::CStr::from_ptr(s.err_msg)
                    .to_string_lossy()
                    .into_owned()
            }
        };
        if let Some(home) = &home {
            let home_field = std::ptr::addr_of_mut!(config.home);
            let s = ffi::PyConfig_SetBytesString(&mut config, home_field, home.as_ptr());
            if ffi::PyStatus_Exception(s) != 0 {
                ffi::PyConfig_Clear(&mut config);
                return Err(status_text(s));
            }
        }
        let s = ffi::Py_InitializeFromConfig(&config);
        ffi::PyConfig_Clear(&mut config);
        if ffi::PyStatus_Exception(s) != 0 {
            return Err(format!(
                "Python failed to start: {} (home {})",
                status_text(s),
                home.as_ref()
                    .map_or("not set".into(), |h| h.to_string_lossy().into_owned())
            ));
        }
        // Let other threads attach (Stop); this thread re-attaches per job.
        ffi::PyEval_SaveThread();
    }
    Ok(())
}

fn spawn() -> Result<Worker, String> {
    let (jobs_tx, jobs_rx) = mpsc::channel::<Job>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(String, u64), String>>();
    std::thread::Builder::new()
        .name("operandi-python".into())
        .spawn(move || {
            if let Err(e) = initialize() {
                let _ = ready_tx.send(Err(e));
                return;
            }
            let ready = Python::attach(|py| {
                setup(py).map_err(|e| traceback(py, &e))?;
                // SAFETY: a plain query of the calling thread's id.
                let ident = unsafe { pyo3::ffi::PyThread_get_thread_ident() } as u64;
                let version = Python::version_str()
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_owned();
                Ok((version, ident))
            });
            let ok = ready.is_ok();
            let _ = ready_tx.send(ready);
            if !ok {
                return;
            }
            for job in jobs_rx {
                run_job(job);
            }
        })
        .map_err(|e| format!("Couldn't start the Python thread: {e}"))?;
    let (version, ident) = ready_rx
        .recv()
        .map_err(|_| "The Python thread stopped while starting".to_owned())??;
    Ok(Worker {
        jobs: Mutex::new(jobs_tx),
        ident: AtomicU64::new(ident),
        busy: AtomicBool::new(false),
        version,
    })
}

/// Installs the `operandi` module and sends print() to the host.
fn setup(py: Python<'_>) -> PyResult<()> {
    let code = CString::new(OPERANDI_PY).expect("no NUL in operandi.py");
    let module = PyModule::from_code(py, &code, c"operandi.py", c"operandi")?;
    py.import("sys")?
        .getattr("modules")?
        .set_item("operandi", module)?;
    py.run(
        c"import sys, _operandi
class _Out:
    def __init__(self, error):
        self.error = error
    def write(self, s):
        _operandi.write(s, self.error)
        return len(s)
    def flush(self):
        pass
    def isatty(self):
        return False
sys.stdout = _Out(False)
sys.stderr = _Out(True)
",
        None,
        None,
    )?;
    let console = PyDict::new(py);
    console.set_item("__name__", "__console__")?;
    console.set_item("__builtins__", py.import("builtins")?)?;
    console.set_item("operandi", py.import("operandi")?)?;
    CONSOLE.with(|c| *c.borrow_mut() = Some(console.unbind()));
    Ok(())
}

fn run_job(job: Job) {
    let Job {
        source,
        msgs,
        replies,
    } = job;
    CURRENT.with(|c| *c.borrow_mut() = Some((msgs.clone(), Some(replies))));
    let outcome = Python::attach(|py| {
        // A Stop that arrived after the last script ended mustn't hit this
        // one. SAFETY: clears this (attached) thread's own pending
        // exception; a null exception is the documented way to clear.
        unsafe {
            let ident = pyo3::ffi::PyThread_get_thread_ident() as c_long;
            pyo3::ffi::PyThreadState_SetAsyncExc(ident, std::ptr::null_mut());
        }
        match execute(py, &source) {
            Ok(()) => Outcome {
                ok: true,
                ..Default::default()
            },
            Err(e) => Outcome {
                ok: false,
                error: Some(traceback(py, &e)),
                ..Default::default()
            },
        }
    });
    CURRENT.with(|c| *c.borrow_mut() = None);
    if let Ok(w) = worker() {
        w.busy.store(false, Ordering::SeqCst);
    }
    let _ = msgs.send(Msg::Done(outcome));
}

fn execute(py: Python<'_>, source: &Source) -> PyResult<()> {
    let builtins = py.import("builtins")?;
    let globals = if source.console {
        CONSOLE
            .with(|c| c.borrow().as_ref().map(|d| d.clone_ref(py).into_bound(py)))
            .unwrap_or_else(|| PyDict::new(py))
    } else {
        let g = PyDict::new(py);
        g.set_item("__name__", "__main__")?;
        g.set_item("__file__", &source.name)?;
        g.set_item("__builtins__", &builtins)?;
        g
    };
    // A console line that is one statement echoes its value ("single");
    // several statements run as a block.
    let compiled = if source.console {
        builtins
            .call_method1("compile", (&source.code, &source.name, "single"))
            .or_else(|_| builtins.call_method1("compile", (&source.code, &source.name, "exec")))?
    } else {
        builtins.call_method1("compile", (&source.code, &source.name, "exec"))?
    };
    builtins.call_method1("exec", (compiled, globals))?;
    Ok(())
}

/// The exception as Python prints it.
fn traceback(py: Python<'_>, e: &PyErr) -> String {
    let formatted = py.import("traceback").and_then(|t| {
        t.call_method1("format_exception", (e.clone_ref(py).into_value(py),))?
            .extract::<Vec<String>>()
    });
    match formatted {
        Ok(lines) => lines.concat(),
        Err(_) => e.to_string(),
    }
}

/// Sends `request` to the host and waits (detached, so Stop can attach)
/// for its reply.
fn call_host(py: Python<'_>, request: Request) -> PyResult<Reply> {
    let link = CURRENT.with(|c| {
        let mut c = c.borrow_mut();
        let (msgs, replies) = c.as_mut()?;
        Some((msgs.clone(), replies.take()?))
    });
    let Some((msgs, replies)) = link else {
        return Err(pyo3::exceptions::PyRuntimeError::new_err(
            "operandi calls work only from the script's own thread",
        ));
    };
    if msgs.send(Msg::Request(request)).is_err() {
        put_back(replies);
        return Err(pyo3::exceptions::PyRuntimeError::new_err(
            "Operandi stopped",
        ));
    }
    let (replies, reply) = py.detach(move || {
        let reply = replies.recv();
        (replies, reply)
    });
    put_back(replies);
    reply.map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Operandi stopped"))
}

fn put_back(replies: Receiver<Reply>) {
    CURRENT.with(|c| {
        if let Some((_, slot)) = c.borrow_mut().as_mut() {
            *slot = Some(replies);
        }
    });
}

fn text_reply(reply: Reply) -> (bool, Option<String>) {
    match reply {
        Reply::Text(t) => (true, t),
        Reply::Failed(e) => (false, Some(e)),
        Reply::Mesh(..) => (true, None),
    }
}

#[pymodule]
fn _operandi(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(call, m)?)?;
    m.add_function(wrap_pyfunction!(write, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_get, m)?)?;
    m.add_function(wrap_pyfunction!(mesh_set, m)?)?;
    Ok(())
}

/// `call(kind, text)`: run, query, group_begin, group_end.
#[pyfunction]
fn call(py: Python<'_>, kind: &str, text: &str) -> PyResult<(bool, Option<String>)> {
    let request = match kind {
        "run" => Request::Run(text.into()),
        "query" => Request::Query(text.into()),
        "group_begin" => Request::GroupBegin(text.into()),
        "group_end" => Request::GroupEnd,
        other => {
            return Err(pyo3::exceptions::PyValueError::new_err(format!(
                "unknown call {other}"
            )));
        }
    };
    Ok(text_reply(call_host(py, request)?))
}

/// print() output, sent to the host as it comes.
#[pyfunction]
fn write(text: &str, error: bool) {
    CURRENT.with(|c| {
        if let Some((msgs, _)) = c.borrow().as_ref() {
            let _ = msgs.send(Msg::Print(Printed {
                text: text.into(),
                error,
            }));
        }
    });
}

type MeshAnswer = (bool, Py<PyAny>, Py<PyAny>, Py<PyAny>);

#[pyfunction]
#[pyo3(signature = (item=None, flat=false))]
fn mesh_get(py: Python<'_>, item: Option<u64>, flat: bool) -> PyResult<MeshAnswer> {
    let none = || py.None();
    match call_host(py, Request::MeshGet(item))? {
        Reply::Mesh(id, data) => {
            let points: Py<PyAny> = if flat {
                let bytes: Vec<u8> = data
                    .points
                    .iter()
                    .flatten()
                    .flat_map(|v| v.to_ne_bytes())
                    .collect();
                pyo3::types::PyBytes::new(py, &bytes).into_any().unbind()
            } else {
                let tuples: Vec<(f64, f64, f64)> =
                    data.points.iter().map(|p| (p[0], p[1], p[2])).collect();
                tuples.into_pyobject(py)?.into_any().unbind()
            };
            let polygons = data.polygons.into_pyobject(py)?.into_any().unbind();
            Ok((
                true,
                id.into_pyobject(py)?.into_any().unbind(),
                points,
                polygons,
            ))
        }
        Reply::Failed(e) => Ok((
            false,
            e.into_pyobject(py)?.into_any().unbind(),
            none(),
            none(),
        )),
        Reply::Text(_) => Ok((
            false,
            "No mesh".into_pyobject(py)?.into_any().unbind(),
            none(),
            none(),
        )),
    }
}

/// Points from a list of (x, y, z) or a flat buffer of doubles.
fn points_of(py: Python<'_>, points: &Bound<'_, PyAny>) -> PyResult<Vec<[f64; 3]>> {
    if let Ok(buffer) = PyBuffer::<f64>::get(points) {
        let flat = buffer.to_vec(py)?;
        if flat.len() % 3 != 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "a flat point buffer holds x, y, z for each point",
            ));
        }
        return Ok(flat.as_chunks::<3>().0.to_vec());
    }
    let list: Vec<Vec<f64>> = points.extract()?;
    list.into_iter()
        .map(|p| match p.as_slice() {
            [x, y, z] => Ok([*x, *y, *z]),
            _ => Err(pyo3::exceptions::PyValueError::new_err(
                "each point is (x, y, z)",
            )),
        })
        .collect()
}

#[pyfunction]
#[pyo3(signature = (points, polygons=None, item=None))]
fn mesh_set(
    py: Python<'_>,
    points: &Bound<'_, PyAny>,
    polygons: Option<&Bound<'_, PyAny>>,
    item: Option<u64>,
) -> PyResult<(bool, Option<String>)> {
    let points = points_of(py, points)?;
    let polygons = polygons
        .filter(|p| !p.is_none())
        .map(|p| p.extract::<Vec<Vec<u32>>>())
        .transpose()?;
    let reply = call_host(
        py,
        Request::MeshSet {
            item,
            points,
            polygons,
        },
    )?;
    Ok(text_reply(reply))
}
