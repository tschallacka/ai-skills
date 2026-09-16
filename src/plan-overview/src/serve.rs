// MODE: DEV
// PACKAGE: PROD
use crate::pages::esc;
use crate::plan::state::parse_state;
use crate::render::router::render_page;
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct ServerHandle {
    address: String,
    stop: Option<Sender<()>>,
    thread: Option<thread::JoinHandle<()>>,
}

#[derive(Clone)]
pub struct StateStream {
    current: Arc<Mutex<String>>,
    clients: Arc<Mutex<Vec<Sender<String>>>>,
}

pub fn state_stream(initial: String) -> StateStream {
    StateStream {
        current: Arc::new(Mutex::new(initial)),
        clients: Arc::new(Mutex::new(Vec::new())),
    }
}

impl StateStream {
    pub fn subscribe(&self) -> Receiver<String> {
        let (sender, receiver) = mpsc::channel();
        let _ = sender.send(self.current.lock().expect("state stream lock").clone());
        self.clients
            .lock()
            .expect("state stream clients lock")
            .push(sender);
        receiver
    }
    pub fn publish(&self, state: String) {
        *self.current.lock().expect("state stream lock") = state.clone();
        self.clients
            .lock()
            .expect("state stream clients lock")
            .retain(|client| client.send(state.clone()).is_ok());
    }
    pub fn current(&self) -> String {
        self.current.lock().expect("state stream lock").clone()
    }
}

impl ServerHandle {
    pub fn address(&self) -> &str {
        &self.address
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn serve(state_stream: StateStream) -> io::Result<ServerHandle> {
    serve_on_host_port(state_stream, "127.0.0.1", 0)
}

pub fn serve_on_port(state_stream: StateStream, port: u16) -> io::Result<ServerHandle> {
    serve_on_host_port(state_stream, "127.0.0.1", port)
}

pub fn serve_on_host_port(
    state_stream: StateStream,
    host: &str,
    port: u16,
) -> io::Result<ServerHandle> {
    let listener = TcpListener::bind((host, port))?;
    listener.set_nonblocking(true)?;
    let address = listener.local_addr()?.to_string();
    println!("{address}");
    io::stdout().flush()?;
    let (stop, stopped) = mpsc::channel();
    let thread = thread::spawn(move || loop {
        if stopped.try_recv().is_ok() {
            break;
        }
        match listener.accept() {
            Ok((stream, _)) => respond(stream, &state_stream),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::sleep(std::time::Duration::from_millis(5))
            }
            Err(_) => break,
        }
    });
    Ok(ServerHandle {
        address,
        stop: Some(stop),
        thread: Some(thread),
    })
}

// Pulls the request path out of an HTTP/1.x request line ("GET /goal/foo
// HTTP/1.1"). Anything malformed or missing the expected shape falls back to
// "/", which route() treats as the overview page -- a client sending garbage
// gets the front page, not a crash.
fn request_path(request: &str) -> &str {
    request
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("GET "))
        .and_then(|rest| rest.split(' ').next())
        .unwrap_or("/")
}

// GET /state reads the stream's CURRENT value on every request rather than a
// value captured at server start, so a --watch-driven publish() (see
// main.rs's run()) is visible to the very next request with no server
// restart. Every other GET is routed and rendered fresh from that same
// current state via router::render_page (T130 plus the routing fix this page
// needed: a request path like /goal/foo or /unit/W01 is real HTTP the
// browser actually sends -- unlike a #hash fragment, which never reaches the
// server at all -- so per-request server-side rendering is what makes every
// link on the page actually navigate, instead of every request silently
// getting back the same snapshot rendered once at server start).
fn respond(mut stream: TcpStream, state_stream: &StateStream) {
    let mut request = [0; 2048];
    let size = stream.read(&mut request).unwrap_or(0);
    let request = String::from_utf8_lossy(&request[..size]);
    let state_json = state_stream.current();
    let (content_type, body) = if request.starts_with("GET /state") {
        ("application/json", state_json)
    } else if request.starts_with("GET /nav.js") {
        (
            "application/javascript",
            include_str!("../assets/nav.js").to_string(),
        )
    } else {
        let path = request_path(&request);
        let page = match parse_state(&state_json) {
            Ok(state) => render_page(&state, path),
            Err(error) => format!(
                "<article><h1>Could not parse plan state</h1><p>{}</p></article>",
                esc(&error.message)
            ),
        };
        ("text/html; charset=utf-8", page)
    };
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.shutdown(Shutdown::Both);
}
