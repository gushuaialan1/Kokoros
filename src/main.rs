mod onn;
mod serve;
mod tts;
mod utils;

use clap::Parser;
use std::net::SocketAddr;
use tts::koko::TTSKoko;

#[derive(Parser, Debug)]
#[command(name = "kokoros")]
#[command(version = "0.1")]
#[command(author = "Lucas Jin")]
struct Cli {
    #[arg(short = 't', long = "text", value_name = "TEXT")]
    text: Option<String>,

    #[arg(
        short = 'l',
        long = "lan",
        value_name = "LANGUAGE",
        help = "https://github.com/espeak-ng/espeak-ng/blob/master/docs/languages.md"
    )]
    lan: Option<String>,

    #[arg(short = 'm', long = "model", value_name = "MODEL")]
    model: Option<String>,

    #[arg(short = 's', long = "style", value_name = "STYLE")]
    style: Option<String>,

    #[arg(long = "oai", value_name = "OpenAI server")]
    oai: bool,

    #[arg(long = "gpu", help = "Enable GPU acceleration")]
    gpu: bool,
}

// 定义一个线程安全的错误类型
#[derive(Debug)]
struct ThreadSafeError(String);

impl std::error::Error for ThreadSafeError {}

impl std::fmt::Display for ThreadSafeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn run_app() -> Result<(), ThreadSafeError> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| ThreadSafeError(e.to_string()))?;
    rt.block_on(async {
        let args = Cli::parse();

        let model_path = args.model.unwrap_or_else(|| "checkpoints/kokoro-v0_19.onnx".to_string());
        let style = args.style.unwrap_or_else(|| "af_sarah.4+af_nicole.6".to_string());
        let lan = args.lan.unwrap_or_else(|| "en-us".to_string());

        let tts = TTSKoko::with_gpu(&model_path, args.gpu);

        if args.oai {
            let app = serve::openai::create_server(tts).await;
            let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
            println!("Starting OpenAI-compatible server on http://localhost:3000");
            axum::serve(
                tokio::net::TcpListener::bind(&addr).await.map_err(|e| ThreadSafeError(e.to_string()))?,
                app.into_make_service(),
            )
            .await
            .map_err(|e| ThreadSafeError(e.to_string()))?;
            Ok(())
        } else {
            let txt = args.text.unwrap_or_else(|| {
                r#"
                Hello, This is Kokoro, your remarkable AI TTS. It's a TTS model with merely 82 million parameters yet delivers incredible audio quality.
This is one of the top notch Rust based inference models, and I'm sure you'll love it. If you do, please give us a star. Thank you very much. 
 As the night falls, I wish you all a peaceful and restful sleep. May your dreams be filled with joy and happiness. Good night, and sweet dreams!
                "#
                .to_string()
            });
            tts.tts(&txt, &lan, &style).map_err(|e| ThreadSafeError(e.to_string()))?;
            Ok(())
        }
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    const STACK_SIZE: usize = 32 * 1024 * 1024; // 32MB 栈大小
    let result = std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(run_app)
        .map_err(|e| Box::new(ThreadSafeError(e.to_string())))?
        .join()
        .map_err(|e| Box::new(ThreadSafeError(format!("Thread panic: {:?}", e))))?;
    
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(Box::new(e))
    }
}
