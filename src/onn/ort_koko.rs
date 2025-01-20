use std::borrow::Cow;

use ndarray::{ArrayBase, IxDyn, OwnedRepr};
use ort::{
    session::{Session, SessionInputValue, SessionInputs, SessionOutputs},
    value::{Tensor, Value},
};

use super::ort_base;
use super::config::OrtConfig;
use ort_base::OrtBase;

pub struct OrtKoko {
    sess: Option<Session>,
    config: OrtConfig,
}

impl ort_base::OrtBase for OrtKoko {
    fn set_sess(&mut self, sess: Session) {
        self.sess = Some(sess);
    }

    fn sess(&self) -> Option<&Session> {
        self.sess.as_ref()
    }
}

impl OrtKoko {
    pub fn new(model_path: String) -> Result<Self, String> {
        Self::with_config(model_path, OrtConfig::default())
    }

    pub fn with_config(model_path: String, config: OrtConfig) -> Result<Self, String> {
        let mut instance = OrtKoko { 
            sess: None,
            config,
        };
        instance.load_model_with_config(model_path, instance.config.clone())?;
        Ok(instance)
    }

    // 提供配置访问方法
    pub fn config(&self) -> &OrtConfig {
        &self.config
    }

    pub fn infer(
        &self,
        tokens: Vec<Vec<i64>>,
        styles: Vec<Vec<f32>>,
    ) -> Result<ArrayBase<OwnedRepr<f32>, IxDyn>, Box<dyn std::error::Error>> {
        println!("\nStarting inference...");
        let start = std::time::Instant::now();

        let shape = [tokens.len(), tokens[0].len()];
        let tokens_flat: Vec<i64> = tokens.into_iter().flatten().collect();
        let tokens = Tensor::from_array((shape, tokens_flat))?;
        let tokens_value: SessionInputValue = SessionInputValue::Owned(Value::from(tokens));

        let shape_style = [styles.len(), styles[0].len()];
        println!("Style shape: {:?}", shape_style);
        let style_flat: Vec<f32> = styles.into_iter().flatten().collect();
        let style = Tensor::from_array((shape_style, style_flat))?;
        let style_value: SessionInputValue = SessionInputValue::Owned(Value::from(style));

        let speed = vec![1.0f32; 1];
        let speed = Tensor::from_array(([1], speed))?;
        let speed_value: SessionInputValue = SessionInputValue::Owned(Value::from(speed));

        let inputs: Vec<(Cow<str>, SessionInputValue)> = vec![
            (Cow::Borrowed("tokens"), tokens_value),
            (Cow::Borrowed("style"), style_value),
            (Cow::Borrowed("speed"), speed_value),
        ];

        if let Some(sess) = &self.sess {
            println!("Running inference with {} tokens...", shape[1]);
            let outputs: SessionOutputs = sess.run(SessionInputs::from(inputs))?;
            let output = outputs["audio"]
                .try_extract_tensor::<f32>()
                .expect("Failed to extract tensor")
                .into_owned();
            
            let duration = start.elapsed();
            println!("✓ Inference completed in {:.2?}", duration);
            if self.config.use_gpu {
                println!("Using GPU acceleration");
            } else {
                println!("Using CPU mode");
            }
            
            Ok(output)
        } else {
            Err("Session is not initialized.".into())
        }
    }
}
