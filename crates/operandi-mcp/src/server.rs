//! Local socket transport for the bridge inside the running app.
//!
//! The app owns the `Editor`, so the socket threads never touch it: each
//! connection thread reads one JSON-RPC line, hands it to the app as a
//! [`Request`], and waits for the app's answer, which the app produces between
//! frames on its own thread. `operandi-mcp --attach` forwards an MCP client's
//! stdio to this socket, so clients keep one launch command.
//!
//! Unix domain socket only, created owner-only (0600): nothing on the network
//! can reach it, and other users on the machine can't either.
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Open client streams by connection number, so `Drop` can close them.
type Connections = Arc<Mutex<Vec<(u64, UnixStream)>>>;

/// Where the app listens unless `OPERANDI_MCP_SOCKET` says otherwise. On
/// macOS the temp dir is per user, which keeps the path short (socket paths
/// are limited to about 100 bytes).
pub fn default_socket_path() -> PathBuf {
    std::env::var_os("OPERANDI_MCP_SOCKET")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("operandi-mcp.sock"))
}

fn lock_path(socket: &Path) -> PathBuf {
    let mut name = socket.as_os_str().to_owned();
    name.push(".lock");
    PathBuf::from(name)
}

/// One JSON-RPC line from a client, waiting for the app's answer.
pub struct Request {
    pub line: String,
    reply: Sender<Option<String>>,
}
impl Request {
    /// Send the answer (`None` for a notification, which gets no reply).
    pub fn respond(self, response: Option<serde_json::Value>) {
        let _ = self.reply.send(response.map(|v| v.to_string()));
    }
}

/// A listening socket. Dropping it stops listening, ends open connections and
/// removes the socket file.
pub struct Server {
    /// Held while serving; see [`Server::start`].
    _lock: std::fs::File,
    path: PathBuf,
    requests: Receiver<Request>,
    stop: Arc<AtomicBool>,
    connections: Connections,
}

impl Server {
    /// Listen at `path`. `wake` is called whenever a request arrives, so the
    /// app can schedule a frame to answer it. Refuses a path another live
    /// server already holds; a stale file from a crash is replaced.
    ///
    /// Liveness is an exclusive lock on `<path>.lock`, held for the server's
    /// lifetime and released by the OS when the process exits or crashes.
    /// Probing the socket file with a connect is not reliable: on macOS a
    /// connect to a leftover file can succeed with nobody listening.
    pub fn start(path: &Path, wake: impl Fn() + Send + Sync + 'static) -> Result<Self, String> {
        let lock_path = lock_path(path);
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .map_err(|e| format!("{}: {e}", lock_path.display()))?;
        // A child process forked while the previous holder closed the file
        // keeps a copy of the lock until it execs, so allow a short wait.
        let mut attempts = 0;
        let locked = loop {
            match lock.try_lock() {
                Err(std::fs::TryLockError::WouldBlock) if attempts < 20 => {
                    attempts += 1;
                    std::thread::sleep(Duration::from_millis(10));
                }
                other => break other,
            }
        };
        match locked {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err(format!(
                    "another Operandi is already serving MCP at {}",
                    path.display()
                ));
            }
            Err(std::fs::TryLockError::Error(e)) => {
                return Err(format!("{}: {e}", lock_path.display()));
            }
        }
        // We hold the lock, so any socket file here is a leftover.
        if std::fs::symlink_metadata(path).is_ok() {
            std::fs::remove_file(path).map_err(|e| format!("{}: {e}", path.display()))?;
        }
        let listener = UnixListener::bind(path).map_err(|e| format!("{}: {e}", path.display()))?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("{}: {e}", path.display()))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let (send, requests) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let connections = Arc::new(Mutex::new(Vec::new()));
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        {
            let stop = stop.clone();
            let connections = connections.clone();
            std::thread::Builder::new()
                .name("operandi-mcp-listen".into())
                .spawn(move || accept_loop(listener, send, stop, connections, wake))
                .map_err(|e| e.to_string())?;
        }
        Ok(Self {
            _lock: lock,
            path: path.to_owned(),
            requests,
            stop,
            connections,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Requests that arrived since the last call, oldest first.
    pub fn take_requests(&self) -> Vec<Request> {
        self.requests.try_iter().collect()
    }

    /// Clients connected right now.
    pub fn client_count(&self) -> usize {
        self.connections.lock().map(|c| c.len()).unwrap_or(0)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Ok(connections) = self.connections.lock() {
            for (_, c) in connections.iter() {
                let _ = c.shutdown(std::net::Shutdown::Both);
            }
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

fn accept_loop(
    listener: UnixListener,
    send: Sender<Request>,
    stop: Arc<AtomicBool>,
    connections: Connections,
    wake: Arc<dyn Fn() + Send + Sync>,
) {
    let mut next = 0u64;
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let _ = stream.set_nonblocking(false);
                let Ok(handle) = stream.try_clone() else {
                    continue;
                };
                next += 1;
                let id = next;
                if let Ok(mut list) = connections.lock() {
                    list.push((id, handle));
                }
                let send = send.clone();
                let wake = wake.clone();
                let connections = connections.clone();
                let _ = std::thread::Builder::new()
                    .name("operandi-mcp-client".into())
                    .spawn(move || {
                        serve_client(stream, send, wake);
                        if let Ok(mut list) = connections.lock() {
                            list.retain(|(i, _)| *i != id);
                        }
                    });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
}

fn serve_client(stream: UnixStream, send: Sender<Request>, wake: Arc<dyn Fn() + Send + Sync>) {
    let Ok(mut out) = stream.try_clone() else {
        return;
    };
    for line in BufReader::new(stream).lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let (reply, answer) = mpsc::channel();
        if send.send(Request { line, reply }).is_err() {
            break;
        }
        wake();
        // The app answers between frames; the channel closes only when the
        // bridge is switched off or the app quits.
        let Ok(response) = answer.recv() else { break };
        if let Some(response) = response
            && (writeln!(out, "{response}").is_err() || out.flush().is_err())
        {
            break;
        }
    }
}

/// `operandi-mcp --attach`: forward stdin lines to the app's socket and its
/// answers to stdout, until either side closes.
pub fn attach(path: &Path) -> Result<(), String> {
    let stream = UnixStream::connect(path).map_err(|e| {
        format!(
            "can't reach a running Operandi at {} ({e}). Is the app open with Window > MCP Bridge on?",
            path.display()
        )
    })?;
    let mut to_app = stream.try_clone().map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let Ok(line) = line else { break };
            if writeln!(to_app, "{line}").is_err() || to_app.flush().is_err() {
                break;
            }
        }
        let _ = to_app.shutdown(std::net::Shutdown::Write);
    });
    let mut out = std::io::stdout().lock();
    for line in BufReader::new(stream).lines() {
        let line = line.map_err(|e| e.to_string())?;
        writeln!(out, "{line}").map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}
