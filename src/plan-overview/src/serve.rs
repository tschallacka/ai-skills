// MODE: DEV
// PACKAGE: PROD
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

pub fn serve(artifact: String, state_stream: StateStream) -> io::Result<ServerHandle> {
    serve_on_port(artifact, state_stream, 0)
}

pub fn serve_on_port(
    artifact: String,
    state_stream: StateStream,
    port: u16,
) -> io::Result<ServerHandle> {
    let listener = TcpListener::bind(("127.0.0.1", port))?;
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
            Ok((stream, _)) => respond(stream, &artifact, &state_stream),
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

// GET /state reads the stream's CURRENT value on every request rather than a
// value captured at server start, so a --watch-driven publish() (see
// main.rs's run()) is visible to the very next request with no server
// restart. The served page itself (GET /) stays the snapshot rendered at
// startup -- T130 only wires the JSON endpoint live, not the HTML shell.
fn respond(mut stream: TcpStream, artifact: &str, state_stream: &StateStream) {
    let mut request = [0; 2048];
    let size = stream.read(&mut request).unwrap_or(0);
    let request = String::from_utf8_lossy(&request[..size]);
    let (content_type, body) = if request.starts_with("GET /state") {
        ("application/json", state_stream.current())
    } else if request.starts_with("GET /nav.js") {
        (
            "application/javascript",
            include_str!("../assets/nav.js").to_string(),
        )
    } else {
        ("text/html; charset=utf-8", artifact.to_string())
    };
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.shutdown(Shutdown::Both);
}
