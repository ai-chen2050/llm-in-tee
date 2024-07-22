use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use sysinfo::System;

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

fn remove_0x_prefix(s: &str) -> &str {
    if s.starts_with("0x") {
        &s[2..]
    } else {
        s
    }
}

pub fn validate_addr(id: &str) -> bool {
    let id = remove_0x_prefix(id);
    if id.len() != 40 {
        return false;
    }

    if !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }

    true
}

pub fn validate_key(id: &str) -> bool {
    let id = remove_0x_prefix(id);
    if id.len() != 64 {
        return false;
    }

    if !id.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }

    true
}

/// return machine using status: (cpu_percent, memory_total, memory_used)
pub fn machine_used() -> (f32, usize, u64, u64) {
    let mut system: System = System::new_all();
    system.refresh_all();
    let cpu_percent = system.global_cpu_info().cpu_usage();
    let cpu_nums = system.cpus().len();
    let memory_total = system.total_memory();
    let memory_used = system.used_memory();
    (cpu_percent, cpu_nums, memory_total, memory_used)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_validate_nodeid() {
        let id = "9c8c905be05044ebeea814781ce9a0580c8fd26228e4605c7e6424c62161f70d";
        assert!(validate_key(id));

        let id = "9c8c905be05044ebeea8";
        assert!(!validate_key(id));
    }

    #[test]
    fn test_remove_0x_prefix() {
        let s1 = "0x123";
        let s2 = "123";
    
        let result1 = remove_0x_prefix(s1);
        let result2 = remove_0x_prefix(s2);

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_machine_used() {
        let ma = machine_used();
        println!("{:?}", ma);
    }
}
