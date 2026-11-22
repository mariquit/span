use crate::Result;

/// Optional relevance scorer. Errors leave the deterministic ranking intact.
pub trait Reranker: Send + Sync {
    fn score(&self, query: &str, documents: &[String]) -> Result<Vec<f64>>;
}
