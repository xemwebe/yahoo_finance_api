//! Minimal scripted HTTP/1.1 server for unit tests.
//!
//! Every request pops the next queued response and the request line is
//! recorded for assertions. Each connection is handled on its own thread so a
//! slow/stuck client does not block the whole mock; dropping the server closes
//! the listener. Compiled only when testing (`#[cfg(test)]` on the module).

use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

type ScriptedResponse = (u16, Vec<(String, String)>, String);

pub(crate) struct MockServer {
    addr: String,
    listener: Option<TcpListener>,
    requests: Arc<Mutex<Vec<String>>>,
    responses: Arc<Mutex<VecDeque<ScriptedResponse>>>,
}

impl MockServer {
    pub(crate) fn start() -> MockServer {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let acceptor = listener.try_clone().expect("failed to clone TcpListener");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let responses = Arc::new(Mutex::new(VecDeque::new()));
        let requests_clone = requests.clone();
        let responses_clone = responses.clone();
        thread::spawn(move || {
            for stream in acceptor.incoming() {
                match stream {
                    Ok(stream) => {
                        let requests = requests_clone.clone();
                        let responses = responses_clone.clone();
                        thread::spawn(move || handle_connection(stream, &requests, &responses));
                    }
                    Err(_) => break,
                }
            }
        });
        MockServer {
            addr,
            listener: Some(listener),
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
        if let Some(listener) = self.listener.take() {
            drop(listener);
        }
    }
}

fn handle_connection(
    stream: TcpStream,
    requests: &Arc<Mutex<Vec<String>>>,
    responses: &Arc<Mutex<VecDeque<ScriptedResponse>>>,
) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line == "\r\n" || line == "\n" {
            break;
        }
    }
    requests
        .lock()
        .unwrap()
        .push(request_line.trim().to_string());
    let (status, headers, body) = responses.lock().unwrap().pop_front().unwrap_or((
        404,
        Vec::new(),
        "not found".to_string(),
    ));
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
