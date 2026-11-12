// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:35.096644
//+ Revised: 	2026/11/12 14:39:56.130884
// -----------------------------------------------------------------------------

//! A tiny HTTP server on 127.0.0.1 for the download tests: serves fixed files, answers 404 for
//! anything else, and can cut a file short (a dropped connection).

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::Arc;

/// What the server sends for one path.
#[derive(Clone)]
pub struct Served {
    pub body: Vec<u8>,
    /// announce this length but send only the body (shorter than announced): a cut connection
    pub announced: Option<usize>,
    /// the HTTP status line after `HTTP/1.1 `
    pub status: &'static str,
    /// extra header lines, such as `"X-RateLimit-Remaining: 0"`
    pub headers: Vec<&'static str>,
}

/// Starts serving `files` (path → content) on a free port; returns the base URL. The server runs
/// until the test process ends.
pub fn serve(files: HashMap<String, Served>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let files = Arc::new(files);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let files = files.clone();
            std::thread::spawn(move || {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut request = String::new();
                if reader.read_line(&mut request).is_err() {
                    return;
                }
                // the rest of the headers, up to the blank line
                let mut line = String::new();
                while reader.read_line(&mut line).is_ok_and(|read| read > 2) {
                    line.clear();
                }
                let path = request.split_whitespace().nth(1).unwrap_or("/").to_owned();
                let _ = match files.get(&path) {
                    Some(served) => {
                        let length = served.announced.unwrap_or(served.body.len());
                        let status = served.status;
                        let extra: String = served.headers.iter().map(|header| format!("{header}\r\n")).collect();
                        let head = format!(
                            "HTTP/1.1 {status}\r\n{extra}Content-Length: {length}\r\nConnection: close\r\n\r\n"
                        );
                        stream.write_all(head.as_bytes()).and_then(|()| stream.write_all(&served.body))
                    }
                    None => {
                        stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    }
                };
            });
        }
    });
    base
}

pub fn file(body: &[u8]) -> Served {
    Served { body: body.to_vec(), announced: None, status: "200 OK", headers: Vec::new() }
}

/// An answer with another status, such as `"403 Forbidden"`.
pub fn answer(status: &'static str, body: &[u8]) -> Served {
    Served { status, ..file(body) }
}
