# Physical Chain (WASM)

A browser-based decentralized "physical chain" where scanning a QR code becomes a proof-of-interaction that appends a block to a local chain. Built with Rust → WebAssembly using Yew, with optional 3D/AR visualization.

## MVP Features
- Local blockchain node in the browser (IndexedDB persistence)
- QR scanning via camera
- Block creation on each valid scan
- 3D chain visualization (Three.js via CDN)

## Roadmap
- P2P sync via WebRTC/libp2p
- WebXR/AR overlays

## Dev Setup
- Prereqs: Rust, wasm-pack, Node.js
- Build/run: see below or use trunk

## Quick Start

### 1) Install tools
- Rust (stable)
- wasm-pack: `cargo install wasm-pack`
- trunk (dev server): `cargo install trunk`

### 2) Run the dev server
```
trunk serve --open
```

### 3) Build for release
```
trunk build --release
```

If trunk isn't available, you can use `wasm-pack build` and serve `index.html` with any static server.
