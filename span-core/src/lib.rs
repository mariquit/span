#[cfg(not(unix))]
compile_error!("Span requires Unix filesystem permissions.");

mod embedding;
mod error;
mod redaction;
mod rerank;
mod store;
mod types;

#[cfg(feature = "fastembed")]
pub use embedding::{BekkoEmbedder, FastEmbedder};
pub use embedding::{Embedder, HashEmbedder};
pub use error::{MemoryError, Result};
pub use rerank::Reranker;
pub use store::MemoryStore;
pub use types::*;
