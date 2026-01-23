# lag-sim

Local internet lag simulator written in Rust. It runs a TCP proxy on 127.0.0.1:9000 and intentionally degrades traffic (delay, drops, throttling, chaos). A small web UI controls the proxy settings.

This is meant for learning how browser traffic flows, including WebSocket upgrade and frame-level passthrough.

## Requirements

- Windows 11 with WSL2 (Ubuntu recommended)
- Node.js 20+
- Rust toolchain (stable)

## Setup

### Install Rust (WSL)

```bash
sudo apt update
sudo apt install -y build-essential curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify:

```bash
rustc --version
cargo --version
```

### Install Node.js (WSL)

```bash
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs
```

Verify:

```bash
node -v
npm -v
```

## Project layout

- `lag-sim/` Rust backend
  - Serves UI + API on `http://127.0.0.1:8080`
  - Runs TCP proxy on `127.0.0.1:9000`
- `lag-sim-ui/` Vue 3 + Tailwind + GSAP frontend

## Build UI

```bash
cd lag-sim-ui
npm install
npm run build
```

This produces `lag-sim-ui/dist`, which the Rust server serves as static files.

## Run

In one terminal:

```bash
cd lag-sim
cargo run
```

Open:

- UI: http://127.0.0.1:8080
- Proxy: 127.0.0.1:9000

## Testing

### 1) Confirm the UI talks to the backend

With the Rust server running:

```bash
curl http://127.0.0.1:8080/stats
```

You should get JSON with `bytes_delayed`, `packets_dropped`, and `goofy`.

Update config:

```bash
curl -X POST http://127.0.0.1:8080/config \
  -H "Content-Type: application/x-www-form-urlencoded" \
  --data "latency=200&drop=10&bandwidth=256&chaos=false"
```

### 2) Route browser traffic through the proxy

You must set your browser (or system) proxy to:

- Host: `127.0.0.1`
- Port: `9000`

Firefox (recommended for testing):
- Settings -> Network Settings -> Manual proxy configuration
- HTTP Proxy: `127.0.0.1`, Port: `9000`
- Enable "Use this proxy server for all protocols"

Then browse normally and adjust sliders in the UI.

Notes:
- HTTPS uses CONNECT tunneling. This proxy is not doing TLS MITM; it only degrades the TCP tunnel.
- Many modern sites use HTTP/2/3 and QUIC; results vary by site and browser.
- WebSocket connections should pass through (handshake + frame passthrough) while being degraded.

### 3) Quick sanity check

Set:
- Latency: 800+
- Drop: 10-30
- Bandwidth: 64-256
- Chaos: on

Open a site that streams lots of requests (news site, image-heavy page) and you should see stats increase and pages stall.

## Notes

This is a learning tool. The proxy implementation is intentionally straightforward and not hardened for production use.
