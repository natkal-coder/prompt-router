pub struct SystemMonitor;

impl SystemMonitor {
    pub fn new() -> Self {
        Self
    }

    pub fn get_cpu_load(&mut self) -> f64 {
        // Stub for now - would use sysinfo or proc filesystem
        // For Phase 1, we'll just return a reasonable default
        0.5 // 50% system load estimate
    }

    pub fn get_memory_usage(&mut self) -> f64 {
        // Stub for now - would use sysinfo or /proc/meminfo
        // For Phase 1, we'll just return a reasonable default
        0.6 // 60% memory usage estimate
    }
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}
