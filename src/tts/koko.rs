use crate::tts::tokenize::tokenize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use ndarray::{ArrayBase, IxDyn, OwnedRepr};

use crate::onn::{OrtBase, OrtKoko, OrtConfig};
use crate::utils;
use crate::utils::fileio::load_json_file;

use espeak_rs::text_to_phonemes;

#[derive(Clone)]
pub struct TTSKoko {
    model_path: String,
    model: Arc<OrtKoko>,
    styles: HashMap<String, [[[f32; 256]; 1]; 511]>,
}

impl TTSKoko {
    const MODEL_URL: &str =
        "https://huggingface.co/hexgrad/Kokoro-82M/resolve/main/kokoro-v0_19.onnx";
    const JSON_DATA_F: &str = "data/voices.json";

    const SAMPLE_RATE: u32 = 24000;

    pub fn new(model_path: &str) -> Self {
        Self::with_gpu(model_path, false)
    }

    pub fn with_gpu(model_path: &str, use_gpu: bool) -> Self {
        let p = Path::new(model_path);
        if !p.exists() {
            utils::fileio::download_file_from_url(TTSKoko::MODEL_URL, model_path)
                .expect("download model failed.");
        } else {
            println!("load model from: {}", model_path);
        }

        let config = if use_gpu {
            println!("Initializing with GPU configuration...");
            OrtConfig::new()
                .with_gpu(true)
                .with_gpu_memory_limit(Some(4 * 1024 * 1024 * 1024))  // 4GB
                .with_cpu_fallback(true)
        } else {
            println!("Initializing with CPU configuration...");
            OrtConfig::default()
        };

        let model = Arc::new(
            OrtKoko::with_config(model_path.to_string(), config)
                .expect("Failed to create Kokoro TTS model"),
        );

        model.print_info();

        let mut instance = TTSKoko {
            model_path: model_path.to_string(),
            model,
            styles: HashMap::new(),
        };
        instance.load_voices();
        instance
    }

    pub fn tts(
        &self,
        txt: &str,
        lan: &str,
        style_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("hello, going to tts. text: {}", txt);

        let txt = txt.trim();
        if txt.is_empty() {
            return Err("Empty text input".into());
        }

        // 先进行音素化处理
        let phonemes = text_to_phonemes(txt, lan, None, false, false)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
            .join("");  // 不添加空格，直接连接
        
        // 在音素序列前后添加标记，确保标记和音素之间没有空格
        let phonemes = format!("${}$", phonemes);
        println!("Generated phonemes: {}", phonemes);
        
        let tokens = vec![tokenize(&phonemes)];
        println!("Tokenized result: {:?}", tokens);
        println!("Tokenized result length: {}", tokens[0].len());

        if let Ok(styles) = self.mix_styles(style_name) {
            let start_t = Instant::now();
            // println!("styles: {:?}", styles);
            let result = self.model.infer(tokens, styles);
            match result {
                Ok(out) => {
                    println!("output: {:?}", out);
                    let phonemes_len = phonemes.len();
                    self.process_and_save_audio(start_t, out, phonemes_len)?;
                }
                Err(e) => {
                    eprintln!("An error occurred during inference: {:?}", e);
                }
            }

            Ok(())
        } else {
            Err(format!("{} failed to parse this style_name.", style_name).into())
        }
    }

    pub fn mix_styles(
        &self,
        style_name: &str,
    ) -> Result<Vec<Vec<f32>>, Box<dyn std::error::Error>> {
        if !style_name.contains("+") {
            if let Some(style) = self.styles.get(style_name) {
                let styles = vec![style[0][0].to_vec()];
                Ok(styles)
            } else {
                Err(format!("can not found from styles_map: {}", style_name).into())
            }
        } else {
            println!("parsing style mix");
            let styles: Vec<&str> = style_name.split('+').collect();

            let mut style_names = Vec::new();
            let mut style_portions = Vec::new();

            for style in styles {
                if let Some((name, portion)) = style.split_once('.') {
                    if let Ok(portion) = portion.parse::<f32>() {
                        style_names.push(name);
                        style_portions.push(portion * 0.1);
                    }
                }
            }
            println!("styles: {:?}, portions: {:?}", style_names, style_portions);

            let mut blended_style = vec![vec![0.0; 256]; 1];

            for (name, portion) in style_names.iter().zip(style_portions.iter()) {
                if let Some(style) = self.styles.get(*name) {
                    let style_slice = &style[0][0]; // This is a [256] array
                                                    // Blend into the blended_style
                    for j in 0..256 {
                        blended_style[0][j] += style_slice[j] * portion;
                    }
                }
            }
            Ok(blended_style)
        }
    }

    fn process_and_save_audio(
        &self,
        start_t: Instant,
        output: ArrayBase<OwnedRepr<f32>, IxDyn>,
        phonemes_len: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Convert output to standard Vec
        let audio: Vec<f32> = output.iter().cloned().collect();

        let audio_duration = audio.len() as f32 / TTSKoko::SAMPLE_RATE as f32;
        let create_duration = start_t.elapsed().as_secs_f32();
        let speedup_factor = audio_duration / create_duration;

        println!(
            "Created audio in length of {:.2}s for {} phonemes in {:.2}s ({:.2}x real-time)",
            audio_duration, phonemes_len, create_duration, speedup_factor
        );

        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: TTSKoko::SAMPLE_RATE,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let mut writer = hound::WavWriter::create("tmp/output.wav", spec)?;

        for &sample in &audio {
            writer.write_sample(sample)?;
        }

        writer.finalize()?;

        println!("Audio saved to tmp/output.wav");
        Ok(())
    }

    pub fn load_voices(&mut self) {
        // load from json, get styles
        let values = load_json_file(TTSKoko::JSON_DATA_F);
        if let Ok(values) = values {
            if let Some(obj) = values.as_object() {
                for (key, value) in obj {
                    // Check if value is an array
                    if let Some(outer_array) = value.as_array() {
                        // Define target multidimensional array
                        let mut array_3d = [[[0.0; 256]; 1]; 511];

                        // Iterate through outer array (511 elements)
                        for (i, inner_value) in outer_array.iter().enumerate() {
                            if let Some(middle_array) = inner_value.as_array() {
                                // Iterate through middle array (1 element)
                                for (j, inner_inner_value) in middle_array.iter().enumerate() {
                                    if let Some(inner_array) = inner_inner_value.as_array() {
                                        // Iterate through inner array (256 elements)
                                        for (k, number) in inner_array.iter().enumerate() {
                                            if let Some(num) = number.as_f64() {
                                                array_3d[i][j][k] = num as f32;
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Insert multidimensional array into HashMap
                        self.styles.insert(key.clone(), array_3d);
                    }
                }
            }

            println!("voice styles loaded: {}", self.styles.len());
            println!("{:?}", self.styles.keys());
            println!(
                "{:?} {:?}",
                self.styles.keys().next(),
                self.styles.keys().nth(1)
            );
        }
    }
}
