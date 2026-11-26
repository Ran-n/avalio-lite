// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:35.096644
//+ Revised: 	2026/11/23 13:00:37.783265
// -----------------------------------------------------------------------------

//! The fixture release server: a tiny HTTP server on 127.0.0.1 with canned replies per route, for
//! the updater tests. A reply can declare another length than its body (a cut connection), send its
//! body in paced pieces (a slow server), or never answer (a hanging one). It counts the requests
//! for each route and keeps every request's headers, so a test can tell what was fetched, how
//! often, and with what.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// `announced` for a reply with no `Content-Length`: its end is the connection closing.
pub const UNKNOWN_LENGTH: usize = usize::MAX;

/// What the server sends for one path.
#[derive(Clone)]
pub struct Served {
    pub body: Vec<u8>,
    /// announce this length but send only the body: a cut connection when shorter, or a declared
    /// size the body doesn't have
    pub announced: Option<usize>,
    /// the HTTP status line after `HTTP/1.1 `
    pub status: &'static str,
    /// extra header lines, such as `"Location: /elsewhere"`
    pub headers: Vec<String>,
    /// send the body `piece` bytes at a time, waiting `pause` before each
    pub paced: Option<(usize, Duration)>,
    /// never answer: the connection stays open and silent
    pub hangs: bool,
}

/// A running server: its base URL, and what it was asked.
#[derive(Clone)]
pub struct Server {
    pub base: String,
    routes: Arc<Mutex<HashMap<String, Served>>>,
    counts: Arc<Mutex<HashMap<String, usize>>>,
    requests: Arc<Mutex<Vec<String>>>,
}

impl Server {
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    /// Serves `served` at `path` from now on (a reply that names the server's own address).
    pub fn set(&self, path: &str, served: Served) {
        self.routes.lock().unwrap().insert(path.to_owned(), served);
    }

    /// How many requests `path` got.
    pub fn count(&self, path: &str) -> usize {
        self.counts.lock().unwrap().get(path).copied().unwrap_or(0)
    }

    /// Every request so far, request line and headers, in order.
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }
}

/// Starts serving `routes` (path → reply) on a free port. The server runs until the test process
/// ends; anything else is a 404.
pub fn serve(routes: HashMap<String, Served>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let server = Server {
        base: format!("http://{}", listener.local_addr().unwrap()),
        routes: Arc::new(Mutex::new(routes)),
        counts: Arc::default(),
        requests: Arc::default(),
    };
    let recorder = server.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let recorder = recorder.clone();
            std::thread::spawn(move || respond(stream, &recorder));
        }
    });
    server
}

/// The 404 reply for a path no route answers.
const NOT_FOUND: &[u8] = b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

/// Reads one request, records it, and sends the route's reply (a 404 when there is none).
fn respond(mut stream: TcpStream, recorder: &Server) {
    let Some((path, whole)) = read_request(&stream) else { return };
    recorder.requests.lock().unwrap().push(whole);
    *recorder.counts.lock().unwrap().entry(path.clone()).or_default() += 1;
    let Some(served) = route_for(recorder, &path) else {
        // the test may have gone already; nobody is left to tell
        let _ = stream.write_all(NOT_FOUND);
        return;
    };
    if served.hangs {
        std::thread::sleep(Duration::from_secs(600));
        return;
    }
    send(&mut stream, &served);
}

/// The reply declared for `path`, or `None` when no route has one.
fn route_for(recorder: &Server, path: &str) -> Option<Served> {
    recorder.routes.lock().unwrap().get(path).cloned()
}

/// The request's path, and its request line and headers as received; `None` when nothing came.
fn read_request(stream: &TcpStream) -> Option<(String, String)> {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request = String::new();
    reader.read_line(&mut request).ok()?;
    let mut whole = request.clone();
    let mut line = String::new();
    while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
        whole.push_str(&line);
        line.clear();
    }
    let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
    Some((path, whole))
}

/// Sends `served`: its head, then its body at once or in paced pieces. A client that hung up ends it.
fn send(stream: &mut TcpStream, served: &Served) {
    if stream.write_all(head_for(served).as_bytes()).is_err() {
        return;
    }
    match served.paced {
        Some((piece, pause)) => write_paced(stream, &served.body, piece, pause),
        // a client that hung up gets nothing more
        None => {
            let _ = stream.write_all(&served.body);
        }
    }
}

/// The status line and headers of `served`, with its announced length (none for an unknown length).
fn head_for(served: &Served) -> String {
    let length = match served.announced {
        Some(UNKNOWN_LENGTH) => String::new(),
        announced => format!("Content-Length: {}\r\n", announced.unwrap_or(served.body.len())),
    };
    let extra: String = served.headers.iter().map(|header| format!("{header}\r\n")).collect();
    format!("HTTP/1.1 {}\r\n{extra}{length}Connection: close\r\n\r\n", served.status)
}

/// Writes `body` in `piece`-byte chunks, waiting `pause` before each; stops when the client hangs up.
fn write_paced(stream: &mut TcpStream, body: &[u8], piece: usize, pause: Duration) {
    for chunk in body.chunks(piece.max(1)) {
        std::thread::sleep(pause);
        if stream.write_all(chunk).and_then(|()| stream.flush()).is_err() {
            return;
        }
    }
}

pub fn file(body: &[u8]) -> Served {
    Served { body: body.to_vec(), announced: None, status: "200 OK", headers: Vec::new(), paced: None, hangs: false }
}

/// An answer with another status, such as `"403 Forbidden"`.
pub fn answer(status: &'static str, body: &[u8]) -> Served {
    Served { status, ..file(body) }
}

/// A route that never answers.
pub fn hanging() -> Served {
    Served { hangs: true, ..file(b"") }
}
