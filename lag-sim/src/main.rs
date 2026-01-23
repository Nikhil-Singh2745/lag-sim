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

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" => "application/javascript; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        _ => "application/octet-stream",
    }
}

async fn serve_proxy(config: Arc<Mutex<LagConfig>>, stats: Arc<Mutex<Stats>>) {
    let listener = TcpListener::bind("127.0.0.1:9000").await.unwrap();
    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let cfg = config.clone();
        let st = stats.clone();
        tokio::spawn(async move {
            if let Err(_) = handle_client(socket, cfg, st).await {
            }
        });
    }
}

async fn handle_client(mut client: TcpStream, config: Arc<Mutex<LagConfig>>, stats: Arc<Mutex<Stats>>) -> tokio::io::Result<()> {
    let mut peek = [0u8; 4096];
    let n = client.peek(&mut peek).await?;
    if n == 0 {
        return Ok(());
    }

    let req_string = String::from_utf8_lossy(&peek[..n]).to_string();
    let (host, port, is_connect) = parse_host_port(&req_string);
    if host.is_empty() {
        return Ok(());
    }

    let mut server = TcpStream::connect(format!("{}:{}", host, port)).await?;
    if is_connect {
        let mut drain = [0u8; 4096];
        let _ = client.read(&mut drain).await?;
        client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n").await?;
    }

    if is_websocket_upgrade(&req_string) {
        let upgraded = handle_ws_handshake(&mut client, &mut server).await?;
        if upgraded {
            let (mut cr, mut cw) = client.into_split();
            let (mut sr, mut sw) = server.into_split();
            let cfg1 = config.clone();
            let st1 = stats.clone();
            let cfg2 = config.clone();
            let st2 = stats.clone();
            let a = tokio::spawn(async move {
                pipe_ws_frames(&mut cr, &mut sw, cfg1, st1).await
            });
            let b = tokio::spawn(async move {
                pipe_ws_frames(&mut sr, &mut cw, cfg2, st2).await
            });
            let _ = tokio::join!(a, b);
            return Ok(());
        }
    }

    let (mut cr, mut cw) = client.into_split();
    let (mut sr, mut sw) = server.into_split();
    let cfg1 = config.clone();
    let st1 = stats.clone();
    let cfg2 = config.clone();
    let st2 = stats.clone();
    let a = tokio::spawn(async move {
        pipe_laggy(&mut cr, &mut sw, cfg1, st1).await
    });
    let b = tokio::spawn(async move {
        pipe_laggy(&mut sr, &mut cw, cfg2, st2).await
    });
    let _ = tokio::join!(a, b);
    Ok(())
}

fn parse_host_port(req: &str) -> (String, u16, bool) {
    let mut host = String::new();
    let mut port = 80;
    let mut is_connect = false;
    if req.starts_with("CONNECT ") {
        is_connect = true;
        if let Some(line) = req.lines().next() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() > 1 {
                let hp = parts[1];
                if let Some((h, p)) = hp.split_once(':') {
                    host = h.to_string();
                    port = p.parse().unwrap_or(443);
                }
            }
        }
    } else {
        for line in req.lines() {
            if line.to_lowercase().starts_with("host:") {
                let v = line.split(':').skip(1).collect::<Vec<_>>().join(":").trim().to_string();
                if let Some((h, p)) = v.split_once(':') {
                    host = h.to_string();
                    port = p.parse().unwrap_or(80);
                } else {
                    host = v;
                }
            }
        }
    }
    (host, port, is_connect)
}

fn is_websocket_upgrade(req: &str) -> bool {
    let mut up = false;
    let mut key = false;
    for line in req.lines() {
        let l = line.to_lowercase();
        if l.starts_with("upgrade:") && l.contains("websocket") {
            up = true;
        }
        if l.starts_with("sec-websocket-key:") {
            key = true;
        }
    }
    up && key
}

async fn handle_ws_handshake(client: &mut TcpStream, server: &mut TcpStream) -> tokio::io::Result<bool> {
    let mut buf = [0u8; 8192];
    let n = client.read(&mut buf).await?;
    if n == 0 {
        return Ok(false);
    }
    server.write_all(&buf[..n]).await?;
    let mut s = [0u8; 8192];
    let m = server.read(&mut s).await?;
    if m == 0 {
        return Ok(false);
    }
    client.write_all(&s[..m]).await?;
    Ok(true)
}

async fn pipe_laggy(reader: &mut (impl AsyncReadExt + Unpin), writer: &mut (impl AsyncWriteExt + Unpin), config: Arc<Mutex<LagConfig>>, stats: Arc<Mutex<Stats>>) {
    let mut buf = [0u8; 8192];
    loop {
        let n = match reader.read(&mut buf).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        if apply_drop(&config, &stats).await {
            continue;
        }
        apply_delay(&config, &stats, n as u64).await;
        apply_bandwidth(&config, n as u64).await;
        if writer.write_all(&buf[..n]).await.is_err() {
            break;
        }
    }
}
// To do list for next time : implement pipe_ws_frames function, currently it's just a placeholder, it doesn't do any lag simulation for websocket frames.
// Also need to implement proper websocket frame parsing and handling and lag simulation logic for that.