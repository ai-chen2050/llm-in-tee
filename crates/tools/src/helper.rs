use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;
use sha2::{Sha256, Digest};

pub fn get_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub fn sha256_str_to_hex(fclock_str: String) -> String {
    let mut f_hasher = Sha256::new();
    f_hasher.update(fclock_str.clone());
    let f_hash_str = f_hasher.finalize();
    let f_hash_hex = format!("{:x}", f_hash_str);
    f_hash_hex
}

pub fn validate_nodeid(id: &str) -> bool {
    if id.len() != 64 {
        return false;
    }

    if !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }

    true
}

/// return machine using status: (cpu_percent, memory_total, memory_used)
pub fn machine_used() -> (f32, u64, u64) {
    let mut system: System = System::new_all();
    system.refresh_all();

    let cpu_percent = system.global_cpu_info().cpu_usage();
    let memory_total = system.total_memory();
    let memory_used = system.used_memory();
    (cpu_percent, memory_total, memory_used)
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_validate_nodeid() {
        let id = "9c8c905be05044ebeea814781ce9a0580c8fd26228e4605c7e6424c62161f70d";
        assert!(validate_nodeid(id));

        let id = "9c8c905be05044ebeea8";
        assert!(!validate_nodeid(id));
    }
}