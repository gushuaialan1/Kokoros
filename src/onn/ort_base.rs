use ort::session::builder::SessionBuilder;
use ort::session::Session;
use ort::execution_providers::CUDAExecutionProvider;
use ort::environment::Environment;

use super::config::OrtConfig;

pub trait OrtBase {
    fn load_model(&mut self, model_path: String) -> Result<(), String> {
        self.load_model_with_config(model_path, OrtConfig::default())
    }

    fn load_model_with_config(&mut self, model_path: String, config: OrtConfig) -> Result<(), String> {
        let mut builder = SessionBuilder::new()
            .map_err(|e| format!("Failed to create session builder: {}", e))?;

        if config.use_gpu {
            println!("Attempting to enable CUDA...");
            
            // 配置 CUDA 提供商
            let cuda_provider = CUDAExecutionProvider::default()
                .with_device_id(0)
                .build();

            // GPU 内存限制通过环境变量设置
            if let Some(limit) = config.gpu_memory_limit {
                std::env::set_var("CUDA_MEMORY_LIMIT", limit.to_string());
                println!("Set CUDA memory limit to {} bytes", limit);
            }

            // 尝试启用 CUDA
            let cuda_result = builder.clone().with_execution_providers([cuda_provider]);
            builder = if let Ok(b) = cuda_result {
                println!("CUDA execution provider successfully enabled");
                println!("GPU Device ID: 0");
                println!("Note: If GPU memory is not showing usage, please check if CUDA toolkit is properly installed");
                b
            } else if config.fallback_to_cpu {
                println!("Failed to enable CUDA, falling back to CPU");
                builder
            } else {
                return Err("Failed to enable CUDA and fallback is disabled".to_string());
            };
        }

        let session = builder
            .commit_from_file(model_path)
            .map_err(|e| format!("Failed to commit from file: {}", e))?;
        
        // 打印会话信息
        println!("Session initialized successfully");
        
        self.set_sess(session);
        Ok(())
    }

    fn print_info(&self) {
        if let Some(session) = self.sess() {
            println!("Input names:");
            for input in &session.inputs {
                println!("  - {}", input.name);
            }
            println!("Output names:");
            for output in &session.outputs {
                println!("  - {}", output.name);
            }
        } else {
            println!("Session is not initialized.");
        }
    }

    fn set_sess(&mut self, sess: Session);
    fn sess(&self) -> Option<&Session>;
}
