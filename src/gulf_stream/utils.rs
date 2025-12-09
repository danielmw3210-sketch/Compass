use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub fn hex_prefix(bytes: &Vec<u8>, n: usize) -> String {
    bytes.iter().take(n / 2).map(|b| format!("{:02x}", b)).collect()
}