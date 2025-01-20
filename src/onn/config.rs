use std::fmt;

#[derive(Debug, Clone)]
pub struct OrtConfig {
    pub use_gpu: bool,
    pub gpu_memory_limit: Option<usize>,
    pub fallback_to_cpu: bool,
}

impl Default for OrtConfig {
    fn default() -> Self {
        Self {
            use_gpu: false,
            gpu_memory_limit: None,
            fallback_to_cpu: true,
        }
    }
}

impl OrtConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_gpu(mut self, enabled: bool) -> Self {
        self.use_gpu = enabled;
        self
    }

    pub fn with_gpu_memory_limit(mut self, limit: Option<usize>) -> Self {
        self.gpu_memory_limit = limit;
        self
    }

    pub fn with_cpu_fallback(mut self, fallback: bool) -> Self {
        self.fallback_to_cpu = fallback;
        self
    }
}

impl fmt::Display for OrtConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "OrtConfig {{ use_gpu: {}, gpu_memory_limit: {:?}, fallback_to_cpu: {} }}",
            self.use_gpu, self.gpu_memory_limit, self.fallback_to_cpu
        )
    }
} 