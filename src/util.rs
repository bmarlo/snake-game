use std::{sync::Mutex, time::{Duration, SystemTime, UNIX_EPOCH}};

static HASH: Mutex<u64> = Mutex::new(0xcbf29ce484222325);

pub fn random_number() -> u64 {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::new(0, 0))
        .subsec_nanos();

    let mut value = HASH.lock().unwrap();
    for i in 0..4 {
        *value *= 0x100000001b3;
        *value ^= ((seed >> (3 - i) * 8) as u8) as u64;
    }

    *value
}
