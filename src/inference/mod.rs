pub mod tokenizer;

use anyhow::Result;
use ndarray::Array2;
use once_cell::sync::OnceCell;
use ort::{
    session::Session,
    value::Value,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use crate::config;
use tokenizer::Tokenizer;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub model: String,
    pub tokens: usize,
    pub inference_time_ms: f64,
}

pub struct EmbeddingModel {
    session: Session,
    tokenizer: Arc<Tokenizer>,
    max_tokens: usize,
    embedding_dim: usize,
    model_name: String,
}

static MODEL: OnceCell<RwLock<EmbeddingModel>> = OnceCell::new();

impl EmbeddingModel {
    pub fn new() -> Result<Self> {
        let settings = config::get_settings();

        // Load tokenizer
        let model_path = Path::new(&settings.model_path);
        let tokenizer = Arc::new(Tokenizer::new(model_path)?);

        // Load ONNX model
        let model_file = model_path.join("model.onnx");

        let session = Session::builder()?
            .with_intra_threads(4)?
            .with_inter_threads(2)?
            .commit_from_file(&model_file)?;

        Ok(EmbeddingModel {
            session,
            tokenizer,
            max_tokens: settings.max_tokens,
            embedding_dim: settings.embedding_dim,
            model_name: settings.model_name.clone(),
        })
    }

    pub fn count_tokens(&self, text: &str) -> usize {
        let tokens = self.tokenizer.encode(text, true);
        tokens.len()
    }

    pub fn encode(&mut self, text: &str, _normalize: bool) -> Result<(Vec<f32>, Metadata)> {
        let start_time = Instant::now();

        // Get model name before any borrows
        let model_name = self.get_model_name();
        let embedding_dim = self.embedding_dim;

        // Tokenize
        let encoding = self.tokenizer.encode_with_attention(text, self.max_tokens);

        // Prepare ONNX inputs
        let batch_size = 1usize;
        let seq_len = encoding.input_ids.len();

        let input_ids = Array2::from_shape_vec(
            (batch_size, seq_len),
            encoding.input_ids.clone(),
        )?;

        let attention_mask = Array2::from_shape_vec(
            (batch_size, seq_len),
            encoding.attention_mask.clone(),
        )?;

        let token_type_ids = Array2::from_shape_vec(
            (batch_size, seq_len),
            encoding.token_type_ids.clone(),
        )?;

        // Convert arrays to Vec and create ORT Values
        let input_ids_vec: Vec<i64> = input_ids.into_raw_vec();
        let attention_mask_vec: Vec<i64> = attention_mask.into_raw_vec();
        let token_type_ids_vec: Vec<i64> = token_type_ids.into_raw_vec();

        let input_ids_value = Value::from_array(([batch_size, seq_len], input_ids_vec))?;
        let attention_mask_value = Value::from_array(([batch_size, seq_len], attention_mask_vec))?;
        let token_type_ids_value = Value::from_array(([batch_size, seq_len], token_type_ids_vec))?;

        // Run inference
        let outputs = self.session.run(ort::inputs![
            "input_ids" => input_ids_value,
            "attention_mask" => attention_mask_value,
            "token_type_ids" => token_type_ids_value,
        ])?;

        // Extract output - returns (shape, data)
        let (_shape, output_data) = outputs["last_hidden_state"]
            .try_extract_tensor::<f32>()?;

        // Mean pooling and L2 normalization (as standalone functions to avoid self borrow)
        let mut embedding = Vec::with_capacity(embedding_dim);
        for _ in 0..embedding_dim {
            embedding.push(0.0f32);
        }

        for i in 0..seq_len {
            let mask = encoding.attention_mask[i] as f32;
            for j in 0..embedding_dim {
                let idx = i * embedding_dim + j;
                embedding[j] += output_data[idx] * mask;
            }
        }

        // Calculate sum of mask
        let mask_sum: f32 = encoding.attention_mask.iter().map(|&x| x as f32).sum();
        let mask_sum = mask_sum.max(1e-9);

        // Divide by mask sum
        for val in embedding.iter_mut() {
            *val /= mask_sum;
        }

        // L2 normalization
        let norm: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
        let norm = norm.max(1e-9);
        for val in embedding.iter_mut() {
            *val /= norm;
        }

        let inference_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;

        let metadata = Metadata {
            model: model_name,
            tokens: encoding.input_ids.len(),
            inference_time_ms: (inference_time_ms * 100.0).round() / 100.0,
        };

        Ok((embedding, metadata))
    }

    fn mean_pooling(&self, embeddings: &[f32], attention_mask: &[i64], seq_len: usize, embedding_dim: usize) -> Vec<f32> {
        let mut result = vec![0.0f32; embedding_dim];

        for i in 0..seq_len {
            let mask = attention_mask[i] as f32;
            for j in 0..embedding_dim {
                let idx = i * embedding_dim + j;
                result[j] += embeddings[idx] * mask;
            }
        }

        // Calculate sum of mask
        let mask_sum: f32 = attention_mask.iter().map(|&x| x as f32).sum();
        let mask_sum = mask_sum.max(1e-9);

        // Divide by mask sum
        for val in result.iter_mut() {
            *val /= mask_sum;
        }

        result
    }

    fn l2_normalize(&self, vec: &[f32]) -> Vec<f32> {
        let norm: f32 = vec.iter().map(|&x| x * x).sum::<f32>().sqrt();
        let norm = norm.max(1e-9);

        vec.iter().map(|&x| x / norm).collect()
    }

    fn get_model_name(&self) -> String {
        self.model_name
            .split('/')
            .last()
            .unwrap_or(&self.model_name)
            .to_string()
    }
}

pub fn init_model() -> Result<()> {
    let model = EmbeddingModel::new()?;
    MODEL
        .set(RwLock::new(model))
        .map_err(|_| anyhow::anyhow!("Model already initialized"))?;
    Ok(())
}

pub fn get_model() -> &'static RwLock<EmbeddingModel> {
    MODEL.get().expect("Model not initialized")
}
