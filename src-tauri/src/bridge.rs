//! Local bridge between the studio and a running preview.
//!
//! The game polls `GET /next?token=…` on 127.0.0.1; the studio pushes at most a
//! few whitelisted commands (`show_screen`, `reload`, `jump`). Nothing is
//! exposed outside the loopback interface, and a random token is required, so a
//! stale bridge file in a project cannot be driven by another process.

use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct Bridge {
    pub port: u16,
    pub token: String,
    queue: Arc<Mutex<VecDeque<String>>>,
    polls: Arc<AtomicUsize>,
    ready: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Bridge {
    pub fn start() -> Result<Bridge, String> {
        let listener = TcpListener::bind("127.0.0.1:0")
            .map_err(|err| format!("Не удалось открыть порт предпросмотра: {err}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|err| format!("Не удалось настроить сокет: {err}"))?;
        let port = listener
            .local_addr()
            .map_err(|err| format!("Не удалось определить порт: {err}"))?
            .port();
        let token = uuid::Uuid::now_v7().simple().to_string();
        let queue: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
        let polls = Arc::new(AtomicUsize::new(0));
        let ready = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));

        let worker_queue = queue.clone();
        let worker_polls = polls.clone();
        let worker_ready = ready.clone();
        let worker_stop = stop.clone();
        let worker_token = token.clone();
        let handle = thread::spawn(move || {
            while !worker_stop.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => handle_request(
                        stream,
                        &worker_token,
                        &worker_queue,
                        &worker_polls,
                        &worker_ready,
                    ),
                    Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(40));
                    }
                    Err(_) => thread::sleep(Duration::from_millis(120)),
                }
            }
        });

        Ok(Bridge {
            port,
            token,
            queue,
            polls,
            ready,
            stop,
            handle: Some(handle),
        })
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/next?token={}", self.port, self.token)
    }

    /// Сколько раз игра спросила мост. Ноль означает, что предпросмотр ещё не
    /// поднялся (или env-переменные не дошли), и команды ставить в очередь рано.
    pub fn polls(&self) -> usize {
        self.polls.load(Ordering::Relaxed)
    }

    /// Игра уже дошла до экрана предпросмотра (а не просто стартовала).
    pub fn ready(&self) -> bool {
        self.ready.load(Ordering::Relaxed)
    }

    pub fn push(&self, command: serde_json::Value) {
        if let Ok(mut queue) = self.queue.lock() {
            // A preview never needs a long backlog: keep the last few commands.
            if queue.len() >= 16 {
                queue.pop_front();
            }
            queue.push_back(command.to_string());
        }
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn handle_request(
    mut stream: TcpStream,
    token: &str,
    queue: &Arc<Mutex<VecDeque<String>>>,
    polls: &Arc<AtomicUsize>,
    ready: &Arc<AtomicBool>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(400)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(400)));
    let mut buffer = [0u8; 2048];
    let Ok(read) = stream.read(&mut buffer) else {
        return;
    };
    let request = String::from_utf8_lossy(&buffer[..read]);
    let first = request.lines().next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");

    if method != "GET" || !target.starts_with("/next") {
        respond(&mut stream, 404, "{}");
        return;
    }
    let expected = format!("token={token}");
    if !query_has_token(target, &expected) {
        respond(&mut stream, 403, "{}");
        return;
    }

    let command = queue
        .lock()
        .ok()
        .and_then(|mut queue| queue.pop_front())
        .unwrap_or_else(|| "{}".to_string());
    polls.fetch_add(1, Ordering::Relaxed);
    if query_has_flag(target, "ready=1") {
        ready.store(true, Ordering::Relaxed);
    }
    respond(&mut stream, 200, &command);
}

fn query_has_token(target: &str, expected: &str) -> bool {
    query_pairs(target).any(|pair| pair == expected)
}

fn query_has_flag(target: &str, flag: &str) -> bool {
    query_pairs(target).any(|pair| pair == flag)
}

fn query_pairs(target: &str) -> impl Iterator<Item = &str> {
    target
        .split_once('?')
        .map(|(_, query)| query.split('&'))
        .into_iter()
        .flatten()
}

fn respond(stream: &mut TcpStream, status: u16, body: &str) {
    let reason = match status {
        200 => "OK",
        403 => "Forbidden",
        _ => "Not Found",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}
