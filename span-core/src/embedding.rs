#[cfg(feature = "fastembed")]
use crate::MemoryError;
use crate::Result;

pub trait Embedder: Send + Sync {
    fn model(&self) -> &str;
    fn version(&self) -> &str;
    fn dimensions(&self) -> usize;
    fn embed(&self, text: &str) -> Result<Vec<f32>>;
}

/// Deterministic, offline feature-hashing vectors. Useful for tests and as a
/// no-download fallback; production semantic search should use FastEmbedder.
pub struct HashEmbedder {
    dimensions: usize,
}

impl HashEmbedder {
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

impl Default for HashEmbedder {
    fn default() -> Self {
        Self::new(384)
    }
}

impl Embedder for HashEmbedder {
    fn model(&self) -> &str {
        "cam-core-feature-hash"
    }
    fn version(&self) -> &str {
        "1"
    }
    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let mut output = vec![0.0; self.dimensions];
        let tokens: Vec<String> = text
            .split(|c: char| !c.is_alphanumeric() && !"_./-".contains(c))
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase)
            .collect();
        for feature in tokens
            .iter()
            .cloned()
            .chain(tokens.windows(2).map(|w| format!("{} {}", w[0], w[1])))
        {
            let digest = sha2::Sha256::digest(feature.as_bytes());
            let index =
                u64::from_le_bytes(digest[0..8].try_into().unwrap()) as usize % self.dimensions;
            let sign = if digest[8] & 1 == 0 { 1.0 } else { -1.0 };
            output[index] += sign;
        }
        normalize(&mut output);
        Ok(output)
    }
}

#[cfg(feature = "fastembed")]
pub struct FastEmbedder {
    model: parking_lot::Mutex<fastembed::TextEmbedding>,
}

#[cfg(feature = "fastembed")]
impl FastEmbedder {
    pub const MODEL_REPOSITORY: &'static str = "Qdrant/all-MiniLM-L6-v2-onnx";
    pub const MODEL_REVISION: &'static str = "d13954661f83248295ba75c1ed411eef3b7b936e";

    pub fn new(cache_dir: Option<std::path::PathBuf>) -> Result<Self> {
        use fastembed::{
            InitOptionsUserDefined, Pooling, TextEmbedding, TokenizerFiles,
            UserDefinedEmbeddingModel,
        };
        use hf_hub::{Repo, RepoType, api::sync::ApiBuilder};
        let cache_dir = cache_dir
            .ok_or_else(|| MemoryError::Embedding("a model cache directory is required".into()))?;
        std::fs::create_dir_all(&cache_dir)?;
        let api = ApiBuilder::new()
            .with_cache_dir(cache_dir)
            .with_progress(false)
            .build()
            .map_err(|error| MemoryError::Embedding(error.to_string()))?;
        let repository = api.repo(Repo::with_revision(
            Self::MODEL_REPOSITORY.into(),
            RepoType::Model,
            Self::MODEL_REVISION.into(),
        ));
        let read = |name: &str| -> Result<Vec<u8>> {
            let path = repository
                .get(name)
                .map_err(|error| MemoryError::Embedding(error.to_string()))?;
            Ok(std::fs::read(path)?)
        };
        let tokenizer_files = TokenizerFiles {
            tokenizer_file: read("tokenizer.json")?,
            config_file: read("config.json")?,
            special_tokens_map_file: read("special_tokens_map.json")?,
            tokenizer_config_file: read("tokenizer_config.json")?,
        };
        let files = UserDefinedEmbeddingModel::new(read("model.onnx")?, tokenizer_files)
            .with_pooling(Pooling::Mean);
        let model =
            TextEmbedding::try_new_from_user_defined(files, InitOptionsUserDefined::default())
                .map_err(|error| MemoryError::Embedding(error.to_string()))?;
        Ok(Self {
            model: parking_lot::Mutex::new(model),
        })
    }
}

#[cfg(feature = "fastembed")]
impl Embedder for FastEmbedder {
    fn model(&self) -> &str {
        Self::MODEL_REPOSITORY
    }
    fn version(&self) -> &str {
        "fastembed-7.1.1+d13954661f83"
    }
    fn dimensions(&self) -> usize {
        384
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let values = self
            .model
            .lock()
            .embed(vec![text], None)
            .map_err(|e| MemoryError::Embedding(e.to_string()))?;
        values
            .into_iter()
            .next()
            .ok_or_else(|| MemoryError::Embedding("embedding model returned no vector".into()))
    }
}

#[cfg(feature = "fastembed")]
pub struct BekkoEmbedder {
    tokenizer: tokenizers::Tokenizer,
    session: parking_lot::Mutex<ort::session::Session>,
}

#[cfg(feature = "fastembed")]
impl BekkoEmbedder {
    pub const MODEL_REPOSITORY: &'static str = "hotchpotch/bekko-static-embedding-v1";
    pub const MODEL_REVISION: &'static str = "c18167b8f244c1bc6de21ce9b710bac8a29c4615";
    const VERSION: &'static str = "int8-512+c18167b8f244c1bc6de21ce9b710bac8a29c4615";
    const DIMENSIONS: usize = 512;
    const MAX_TOKENS: usize = 32_768;

    pub fn new(cache_dir: std::path::PathBuf) -> Result<Self> {
        use hf_hub::{Repo, RepoType, api::sync::ApiBuilder};
        use ort::session::Session;
        use tokenizers::{Tokenizer, TruncationParams};

        std::fs::create_dir_all(&cache_dir)?;
        let api = ApiBuilder::new()
            .with_cache_dir(cache_dir)
            .with_progress(false)
            .build()
            .map_err(bekko_error)?;
        let repo = api.repo(Repo::with_revision(
            Self::MODEL_REPOSITORY.into(),
            RepoType::Model,
            Self::MODEL_REVISION.into(),
        ));
        let mut tokenizer = Tokenizer::from_file(repo.get("tokenizer.json").map_err(bekko_error)?)
            .map_err(bekko_error)?;
        tokenizer.with_padding(None);
        tokenizer
            .with_truncation(Some(TruncationParams {
                max_length: Self::MAX_TOKENS,
                ..Default::default()
            }))
            .map_err(bekko_error)?;
        let model_path = repo
            .get("onnx/model_int8_dim512.onnx")
            .map_err(bekko_error)?;
        let session = Session::builder()
            .map_err(bekko_error)?
            .with_intra_threads(1)
            .map_err(bekko_error)?
            .commit_from_file(model_path)
            .map_err(bekko_error)?;
        Ok(Self {
            tokenizer,
            session: parking_lot::Mutex::new(session),
        })
    }
}

#[cfg(feature = "fastembed")]
impl Embedder for BekkoEmbedder {
    fn model(&self) -> &str {
        Self::MODEL_REPOSITORY
    }

    fn version(&self) -> &str {
        Self::VERSION
    }

    fn dimensions(&self) -> usize {
        Self::DIMENSIONS
    }

    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        use ort::value::Tensor;

        let encoding = self.tokenizer.encode(text, false).map_err(bekko_error)?;
        let mut ids: Vec<i64> = encoding.get_ids().iter().copied().map(i64::from).collect();
        let mut mask: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .copied()
            .map(i64::from)
            .collect();
        if ids.is_empty() {
            // Match the upstream loader's masked padding for empty inputs.
            ids.push(0);
            mask.push(0);
        }
        let shape = [1, ids.len()];
        let ids = Tensor::from_array((shape, ids)).map_err(bekko_error)?;
        let mask = Tensor::from_array((shape, mask)).map_err(bekko_error)?;
        let mut session = self.session.lock();
        let outputs = session
            .run(ort::inputs!["input_ids" => ids, "attention_mask" => mask])
            .map_err(bekko_error)?;
        let output = outputs
            .get("sentence_embedding")
            .ok_or_else(|| bekko_error("missing sentence_embedding output"))?;
        let (shape, values) = output.try_extract_tensor::<f32>().map_err(bekko_error)?;
        if **shape != [1, Self::DIMENSIONS as i64] || !values.iter().all(|value| value.is_finite())
        {
            return Err(bekko_error(
                "expected one finite 512-dimensional sentence embedding",
            ));
        }
        let mut vector = values.to_vec();
        normalize(&mut vector);
        Ok(vector)
    }
}

#[cfg(feature = "fastembed")]
fn bekko_error(error: impl std::fmt::Display) -> MemoryError {
    MemoryError::Embedding(format!("Bekko Static: {error}"))
}

pub(crate) fn normalize(vector: &mut [f32]) {
    let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vector {
            *value /= norm;
        }
    }
}

pub(crate) fn cosine(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    a.iter()
        .zip(b)
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum()
}

use sha2::Digest;
