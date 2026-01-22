use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::time::Duration;
use std::path::{Path, PathBuf};
use rand::Rng;

#[derive(Clone)]
struct LagConfig {
    latency_ms: u64,
    drop_pct: u8,
    bandwidth_kbps: u64,
    chaos: bool,
}

#[derive(Clone)]
struct Stats {
    bytes_delayed: u64,
    packets_dropped: u64,
    goofy: String,
}

#[tokio::main]
async fn main() {
    let config = Arc::new(Mutex::new(LagConfig {
        latency_ms: 120,
        drop_pct: 5,
        bandwidth_kbps: 256,
        chaos: false,
    }));
    let stats = Arc::new(Mutex::new(Stats {
        bytes_delayed: 0,
        packets_dropped: 0,
        goofy: "NETWORK IS HAVING A BAD DAY".to_string(),
    }));

    let api_cfg = config.clone();
    let api_stats = stats.clone();
    tokio::spawn(async move {
        serve_api_and_ui(api_cfg, api_stats).await;
    });

    let proxy_cfg = config.clone();
    let proxy_stats = stats.clone();
    serve_proxy(proxy_cfg, proxy_stats).await;
}

async fn serve_api_and_ui(config: Arc<Mutex<LagConfig>>, stats: Arc<Mutex<Stats>>) {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    let dist_root = PathBuf::from("../lag-sim-ui/dist");

    loop {
        let (mut socket, _) = listener.accept().await.unwrap();
        let cfg = config.clone();
        let st = stats.clone();
        let dist_root = dist_root.clone();
        tokio::spawn(async move {
            let mut buf = [0u8; 8192];
            let n = socket.read(&mut buf).await.unwrap_or(0);
            if n == 0 {
                return;
            }

            let req = String::from_utf8_lossy(&buf[..n]);
            let mut parts = req.lines().next().unwrap_or("").split_whitespace();
            let method = parts.next().unwrap_or("");
            let path = parts.next().unwrap_or("/");

            if method == "GET" && path == "/stats" {
                let s = st.lock().await;
                let body = format!(
                    "{{\"bytes_delayed\":{},\"packets_dropped\":{},\"goofy\":\"{}\"}}",
                    s.bytes_delayed, s.packets_dropped, s.goofy
                );
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(resp.as_bytes()).await;
                return;
            }

            if method == "POST" && path == "/config" {
                if let Some(body) = req.split("\r\n\r\n").nth(1) {
                    let mut c = cfg.lock().await;
                    for part in body.split('&') {
                        let mut it = part.split('=');
                        let k = it.next().unwrap_or("");
                        let v = it.next().unwrap_or("");
                        match k {
                            "latency" => c.latency_ms = v.parse().unwrap_or(c.latency_ms),
                            "drop" => c.drop_pct = v.parse().unwrap_or(c.drop_pct),
                            "bandwidth" => c.bandwidth_kbps = v.parse().unwrap_or(c.bandwidth_kbps),
                            "chaos" => c.chaos = v == "true",
                            _ => {}
                        }
                    }
                    let mut s = st.lock().await;
                    s.goofy = random_goofy();
                }
                let resp = "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n";
                let _ = socket.write_all(resp.as_bytes()).await;
                return;
            }

            if method != "GET" {
                let resp = "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\n\r\n";
                let _ = socket.write_all(resp.as_bytes()).await;
                return;
            }

            // Static file serving (Vue dist)
            let safe_path = if path == "/" { "/index.html" } else { path };
            if safe_path.contains("..") {
                let resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n";
                let _ = socket.write_all(resp.as_bytes()).await;
                return;
            }

            let candidate = dist_root.join(safe_path.trim_start_matches('/'));
            let file_path = if candidate.exists() {
                candidate
            } else {
                dist_root.join("index.html")
            };

            match tokio::fs::read(&file_path).await {
                Ok(contents) => {
                    let mime = content_type(&file_path);
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
                        mime,
                        contents.len()
                    );
                    let _ = socket.write_all(resp.as_bytes()).await;
                    let _ = socket.write_all(&contents).await;
                }
                Err(_) => {
                    let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                    let _ = socket.write_all(resp.as_bytes()).await;
                }
            }
        });
    }
}

//Looooots to do