use ort::session::builder::{SessionBuilder, GraphOptimizationLevel};
use ort::session::Session;
use ort::execution_providers::CUDAExecutionProvider;
use ort::environment::Environment;

use super::config::OrtConfig;

pub trait OrtBase {
    fn load_model(&mut self, model_path: String) -> Result<(), String> {
        self.load_model_with_config(model_path, OrtConfig::default())
    }

    fn load_model_with_config(&mut self, model_path: String, config: OrtConfig) -> Result<(), String> {
        // 设置 CUDA 环境变量
        if config.use_gpu {
            println!("Setting up CUDA environment variables...");
            std::env::set_var("CUDA_VISIBLE_DEVICES", "0");  // 强制使用第一个 NVIDIA GPU
            std::env::set_var("CUDA_MODULE_LOADING", "LAZY");
            std::env::set_var("ORT_CUDA_PROVIDER_OPTIONS", "cudnn_conv_algo_search=DEFAULT");
            if let Some(limit) = config.gpu_memory_limit {
                std::env::set_var("CUDA_MEMORY_LIMIT", limit.to_string());
                println!("Set CUDA memory limit to {} GB", limit / (1024 * 1024 * 1024));
            }
        }

        let mut builder = SessionBuilder::new()
            .map_err(|e| format!("Failed to create session builder: {}", e))?;

        // 设置优化级别
        builder = builder
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| format!("Failed to set optimization level: {}", e))?;

        // 设置线程数
        builder = builder
            .with_intra_threads(1)
            .map_err(|e| format!("Failed to set thread count: {}", e))?;

        if config.use_gpu {
            println!("Attempting to enable CUDA for NVIDIA GPU...");
            
            // 配置 CUDA 提供商
            let cuda_provider = CUDAExecutionProvider::default()
                .with_device_id(0)  // 使用第一个 NVIDIA GPU
                .build();

            // 尝试启用 CUDA
            let cuda_result = builder.clone().with_execution_providers([cuda_provider]);
            builder = if let Ok(b) = cuda_result {
                println!("✓ CUDA execution provider successfully enabled");
                println!("✓ GPU Device ID: 0");
                println!("✓ Optimization Level: Level3");
                println!("✓ Memory Limit: {} GB", config.gpu_memory_limit.unwrap_or(0) / (1024 * 1024 * 1024));
                b
            } else if config.fallback_to_cpu {
                println!("⚠ Failed to enable CUDA for NVIDIA GPU, falling back to CPU");
                println!("Please check:");
                println!("1. If NVIDIA CUDA Toolkit is properly installed");
                println!("2. Run 'nvidia-smi' to verify GPU is detected");
                println!("3. Check if CUDA version matches the ONNX Runtime version");
                builder
            } else {
                return Err("Failed to enable CUDA and fallback is disabled".to_string());
            };
        }

        println!("Loading model from: {}", model_path);
        let session = builder
            .commit_from_file(model_path)
            .map_err(|e| format!("Failed to commit from file: {}", e))?;
        
        println!("✓ Session initialized successfully");
        
        self.set_sess(session);
        Ok(())
    }

    fn print_info(&self) {
        if let Some(session) = self.sess() {
            println!("\nModel Information:");
            println!("----------------");
            println!("Input names:");
            for input in &session.inputs {
                println!("  - {}", input.name);
            }
            println!("Output names:");
            for output in &session.outputs {
                println!("  - {}", output.name);
            }
            println!("----------------\n");
        } else {
            println!("Session is not initialized.");
        }
    }

    fn set_sess(&mut self, sess: Session);
    fn sess(&self) -> Option<&Session>;
}
