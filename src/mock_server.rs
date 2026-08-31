//! Minimal scripted HTTP/1.1 server for unit tests.
//!
//! Every request pops the next queued response and the request line is
//! recorded for assertions. Each connection is handled on its own thread so a
//! slow/stuck client does not block the whole mock; dropping the server stops
//! the accept loop and joins its thread. Compiled only when testing
//! (`#[cfg(test)]` on the module).

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

type ScriptedResponse = (u16, Vec<(String, String)>, String);

pub(crate) struct MockServer {
    addr: String,
    /// Owned by the accept thread (the listener must not be cloned, otherwise
    /// closing it from `Drop` would not unblock the accepting thread).
    stop: Arc<AtomicBool>,
    accept_thread: Option<thread::JoinHandle<()>>,
    requests: Arc<Mutex<Vec<String>>>,
    responses: Arc<Mutex<VecDeque<ScriptedResponse>>>,
}

impl MockServer {
    pub(crate) fn start() -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        // Non-blocking so the accept loop can poll the stop flag and exit
        // without leaking a thread per server instance.
        listener.set_nonblocking(true).expect("set nonblocking");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(VecDeque::new()));
        let requests_clone = requests.clone();
        let responses_clone = responses.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = stop.clone();
        let accept_thread = thread::spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let requests = requests_clone.clone();
                        let responses = responses_clone.clone();
                        thread::spawn(move || handle_connection(stream, &requests, &responses));
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if stop_clone.load(Ordering::Relaxed) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        MockServer {
            addr,
            stop,
            accept_thread: Some(accept_thread),
            requests,
            responses,
        }
    }

    pub(crate) fn addr(&self) -> &str {
        &self.addr
    }

    pub(crate) fn cookie_url(&self) -> String {
        format!("http://{}/", self.addr())
    }

    pub(crate) fn chart_url(&self) -> String {
        format!("http://{}/v8/finance/chart", self.addr())
    }

    pub(crate) fn crumb_url(&self) -> String {
        format!("http://{}/v1/test/getcrumb", self.addr())
    }

    pub(crate) fn summary_url(&self) -> String {
        format!("http://{}/v10/finance/quoteSummary", self.addr())
    }

    pub(crate) fn earnings_url(&self) -> String {
        format!("http://{}/v1/finance/visualization", self.addr())
    }

    /// Queue a response: (status, extra headers, body)
    pub(crate) fn enqueue(&self, status: u16, headers: &[(&str, &str)], body: &str) {
        self.responses.lock().unwrap().push_back((
            status,
            headers
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body.to_string(),
        ));
    }

    pub(crate) fn enqueue_plain(&self, status: u16, body: &str) {
        self.enqueue(status, &[], body);
    }

    pub(crate) fn request_lines(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.accept_thread.take() {
            handle.join().expect("mock server accept thread panicked");
        }
    }
}

fn handle_connection(
    stream: TcpStream,
    requests: &Arc<Mutex<Vec<String>>>,
    responses: &Arc<Mutex<VecDeque<ScriptedResponse>>>,
) {
    // The listener is non-blocking and accepted sockets inherit O_NONBLOCK on
    // Linux; clear it so the reads below block until data (or EOF) arrive.
    // A read timeout reaps clients that connect and then send nothing.
    stream
        .set_nonblocking(false)
        .and_then(|_| stream.set_read_timeout(Some(Duration::from_millis(2_000))))
        .unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    // Ok(0) means EOF reached with no data: a half-open connection must not
    // spin the loop.
    if reader
        .read_line(&mut request_line)
        .map(|n| n == 0)
        .unwrap_or(true)
    {
        return;
    }
    loop {
        let mut line = String::new();
        let read = match reader.read_line(&mut line) {
            Ok(n) => n,
            Err(_) => break,
        };
        if read == 0 || line == "\r\n" || line == "\n" {
            break;
        }
    }
    requests
        .lock()
        .unwrap()
        .push(request_line.trim().to_string());
    let (status, headers, body) =
        responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or((404, Vec::new(), "not found".to_string()));
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        _ => "Status",
    };
    let mut response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    for (key, value) in headers {
        response.push_str(&format!("{key}: {value}\r\n"));
    }
    response.push_str("\r\n");
    response.push_str(&body);
    let _ = stream.try_clone().unwrap().write_all(response.as_bytes());
}
