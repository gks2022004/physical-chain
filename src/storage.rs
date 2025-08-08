use gloo::storage::{LocalStorage, Storage};
use serde::{Deserialize, Serialize};

use crate::blockchain::Chain;

const KEY: &str = "physical_chain_v1";

#[derive(Serialize, Deserialize)]
pub struct Persisted {
    pub chain: Chain,
}

pub fn load_chain() -> Chain {
    if let Ok(s) = LocalStorage::get::<String>(KEY) {
        if let Ok(p) = serde_json::from_str::<Persisted>(&s) {
            return p.chain;
        }
    }
    Chain::new()
}

pub fn save_chain(chain: &Chain) {
    let p = Persisted { chain: chain.clone() };
    if let Ok(s) = serde_json::to_string(&p) {
        let _ = LocalStorage::set(KEY, s);
    }
}
