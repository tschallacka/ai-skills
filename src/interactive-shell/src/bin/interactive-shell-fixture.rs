use std::io::{self, Write};
use std::thread;
use std::time::Duration;

fn main() {
    let mut stdout = io::stdout();
    stdout.write_all(b"\x1b[").unwrap();
    stdout.flush().unwrap();
    thread::sleep(Duration::from_millis(30));
    stdout.write_all(&[b'1'; 129]).unwrap();
    stdout.flush().unwrap();
    thread::sleep(Duration::from_millis(30));
    stdout.write_all(b"mCSI_SAFE\x1b]").unwrap();
    stdout.flush().unwrap();
    thread::sleep(Duration::from_millis(30));
    stdout.write_all(&[b'x'; 4097]).unwrap();
    stdout.write_all(b"\x07OSC_SAFE").unwrap();
    stdout.flush().unwrap();
    thread::sleep(Duration::from_millis(200));
}
