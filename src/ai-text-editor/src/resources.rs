// MODE: DEV
// PACKAGE: PROD
use serde::Serialize;

pub const SERVER_BASE_OVERHEAD_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize)]
pub struct ResourceReport {
    pub available_memory_bytes: Option<u64>,
    pub estimated_server_overhead_bytes: u64,
    pub recommended_working_set_bytes: Option<u64>,
    pub large_file_threshold_bytes: u64,
}

pub fn report(document_bytes: usize, large_file_threshold_bytes: u64) -> ResourceReport {
    let available = available_memory_bytes();
    let overhead = SERVER_BASE_OVERHEAD_BYTES.saturating_add(document_bytes as u64);
    let recommended = available.map(|value| value.saturating_sub(overhead).min(value / 4));
    ResourceReport {
        available_memory_bytes: available,
        estimated_server_overhead_bytes: overhead,
        recommended_working_set_bytes: recommended,
        large_file_threshold_bytes,
    }
}

fn available_memory_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let content = std::fs::read_to_string("/proc/meminfo").ok()?;
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("MemAvailable:") {
                let kib = value.split_whitespace().next()?.parse::<u64>().ok()?;
                return kib.checked_mul(1024);
            }
        }
    }
    None
}
