use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
// use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Block {
    pub index: u64,
    pub timestamp: f64,
    pub prev_hash: String,
    pub nonce: u64,
    pub data: Interaction,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Interaction {
    pub qr_content: String,
    pub device_hash: String,
    pub geolocation: Option<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Chain {
    pub blocks: Vec<Block>,
}

impl Chain {
    pub fn new() -> Self {
        let mut c = Self { blocks: vec![] };
        if c.blocks.is_empty() {
            c.blocks.push(genesis());
        }
        c
    }

    pub fn last_hash(&self) -> String {
        self.blocks.last().map(|b| b.hash.clone()).unwrap_or_default()
    }

    pub fn add_block(&mut self, data: Interaction, timestamp: f64) -> &Block {
        let index = self.blocks.len() as u64;
        let prev_hash = self.last_hash();
        let (nonce, hash) = mine(index, timestamp, &prev_hash, &data);
        let block = Block { index, timestamp, prev_hash, nonce, data, hash };
        self.blocks.push(block);
        self.blocks.last().unwrap()
    }

    pub fn is_valid(&self) -> bool {
        for i in 1..self.blocks.len() {
            let prev = &self.blocks[i - 1];
            let cur = &self.blocks[i];
            if cur.prev_hash != prev.hash { return false; }
            let computed = hash_block(cur.index, cur.timestamp, &cur.prev_hash, cur.nonce, &cur.data);
            if computed != cur.hash { return false; }
            if !valid_pow(&cur.hash) { return false; }
        }
        true
    }

    pub fn has_qr_content(&self, content: &str) -> bool {
        self.blocks.iter().any(|b| b.data.qr_content == content)
    }
}

fn hash_block(index: u64, timestamp: f64, prev_hash: &str, nonce: u64, data: &Interaction) -> String {
    let payload = serde_json::json!({
        "index": index,
        "timestamp": timestamp,
        "prev_hash": prev_hash,
        "nonce": nonce,
        "data": data,
    });
    let mut hasher = Sha256::new();
    hasher.update(payload.to_string().as_bytes());
    let out = hasher.finalize();
    hex::encode(out)
}

fn valid_pow(h: &str) -> bool {
    // Very light PoW for demo: 2 leading zeros in hex
    h.starts_with("00")
}

fn mine(index: u64, timestamp: f64, prev_hash: &str, data: &Interaction) -> (u64, String) {
    let mut nonce = 0u64;
    loop {
        let h = hash_block(index, timestamp, prev_hash, nonce, data);
        if valid_pow(&h) { return (nonce, h); }
        nonce = nonce.wrapping_add(1);
    // In wasm single-threaded, keep PoW trivial to avoid jank.
    }
}

fn genesis() -> Block {
    let data = Interaction { qr_content: "genesis".into(), device_hash: "0".into(), geolocation: None };
    let timestamp = 0.0;
    let (nonce, hash) = mine(0, timestamp, "", &data);
    Block { index: 0, timestamp, prev_hash: "".into(), nonce, data, hash }
}
