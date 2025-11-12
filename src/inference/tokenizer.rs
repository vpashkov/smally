use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Encoding {
    pub input_ids: Vec<i64>,
    pub attention_mask: Vec<i64>,
    pub token_type_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
struct TokenizerConfig {
    #[serde(default = "default_lowercase")]
    do_lower_case: bool,
}

fn default_lowercase() -> bool {
    true
}

pub struct Tokenizer {
    vocab: HashMap<String, i64>,
    ids_to_tokens: HashMap<i64, String>,
    cls_token_id: i64,
    sep_token_id: i64,
    pad_token_id: i64,
    unk_token_id: i64,
    do_lower_case: bool,
}

impl Tokenizer {
    pub fn new(model_path: &Path) -> Result<Self> {
        let vocab_path = model_path.join("vocab.txt");
        let config_path = model_path.join("tokenizer_config.json");

        // Load vocab
        let (vocab, ids_to_tokens) = Self::load_vocab(&vocab_path)?;

        // Load config
        let config = Self::load_config(&config_path);

        Ok(Tokenizer {
            cls_token_id: *vocab.get("[CLS]").unwrap_or(&101),
            sep_token_id: *vocab.get("[SEP]").unwrap_or(&102),
            pad_token_id: *vocab.get("[PAD]").unwrap_or(&0),
            unk_token_id: *vocab.get("[UNK]").unwrap_or(&100),
            vocab,
            ids_to_tokens,
            do_lower_case: config.do_lower_case,
        })
    }

    pub fn encode(&self, text: &str, add_special_tokens: bool) -> Vec<i64> {
        let tokens = self.tokenize(text);
        let mut ids = Vec::with_capacity(tokens.len() + 2);

        if add_special_tokens {
            ids.push(self.cls_token_id);
        }

        for token in tokens {
            ids.push(*self.vocab.get(&token).unwrap_or(&self.unk_token_id));
        }

        if add_special_tokens {
            ids.push(self.sep_token_id);
        }

        ids
    }

    pub fn encode_with_attention(&self, text: &str, max_length: usize) -> Encoding {
        let mut ids = self.encode(text, true);

        // Truncate if needed
        if ids.len() > max_length {
            ids.truncate(max_length - 1);
            ids.push(self.sep_token_id);
        }

        // Create attention mask
        let mut attention_mask = vec![1i64; ids.len()];

        // Pad to max length
        while ids.len() < max_length {
            ids.push(self.pad_token_id);
            attention_mask.push(0);
        }

        // Token type IDs (all 0s for single sequence)
        let token_type_ids = vec![0i64; max_length];

        Encoding {
            input_ids: ids,
            attention_mask,
            token_type_ids,
        }
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        let text = if self.do_lower_case {
            text.to_lowercase()
        } else {
            text.to_string()
        };

        let text = text.trim();

        // Basic whitespace tokenization + WordPiece
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut tokens = Vec::new();

        for word in words {
            tokens.extend(self.wordpiece(word));
        }

        tokens
    }

    fn wordpiece(&self, word: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut start = 0;

        while start < word.len() {
            let mut end = word.len();
            let mut found = false;

            while end > start {
                let substr = if start > 0 {
                    format!("##{}", &word[start..end])
                } else {
                    word[start..end].to_string()
                };

                if self.vocab.contains_key(&substr) {
                    tokens.push(substr);
                    found = true;
                    break;
                }

                // Move back by character boundary
                end = word[..end].char_indices().rev().next().map(|(i, _)| i).unwrap_or(0);
            }

            if !found {
                tokens.push("[UNK]".to_string());
                break;
            }

            start = end;
        }

        tokens
    }

    fn load_vocab(path: &Path) -> Result<(HashMap<String, i64>, HashMap<i64, String>)> {
        let content = fs::read_to_string(path)?;
        let mut vocab = HashMap::new();
        let mut ids_to_tokens = HashMap::new();

        for (i, line) in content.lines().enumerate() {
            let token = line.trim();
            if !token.is_empty() {
                vocab.insert(token.to_string(), i as i64);
                ids_to_tokens.insert(i as i64, token.to_string());
            }
        }

        Ok((vocab, ids_to_tokens))
    }

    fn load_config(path: &Path) -> TokenizerConfig {
        fs::read_to_string(path)
            .ok()
            .and_then(|content| serde_json::from_str(&content).ok())
            .unwrap_or(TokenizerConfig { do_lower_case: true })
    }
}
