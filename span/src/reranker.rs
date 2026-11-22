use serde::Deserialize;
use span_core::{MemoryError, Reranker, Result};

pub struct HttpReranker {
    client: reqwest::blocking::Client,
    endpoint: String,
    model: String,
    token: Option<String>,
}

impl HttpReranker {
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        let Ok(endpoint) = std::env::var("SPAN_RERANK_URL") else {
            return Ok(None);
        };
        let url = reqwest::Url::parse(&endpoint)?;
        anyhow::ensure!(
            url.scheme() == "https"
                || (url.scheme() == "http"
                    && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))),
            "Reranker requires HTTPS or loopback HTTP"
        );
        Ok(Some(Self::new(
            endpoint,
            std::env::var("SPAN_RERANK_MODEL").unwrap_or_else(|_| "reranker".into()),
            std::env::var("SPAN_RERANK_TOKEN").ok(),
        )?))
    }

    fn new(endpoint: String, model: String, token: Option<String>) -> anyhow::Result<Self> {
        Ok(Self {
            client: reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .redirect(reqwest::redirect::Policy::none())
                .build()?,
            endpoint,
            model,
            token,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        net::TcpListener,
        time::Duration,
    };

    #[test]
    fn reranker_reuses_the_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let reranker =
            HttpReranker::new(format!("http://{address}/rerank"), "test".into(), None).unwrap();
        let server = std::thread::spawn(move || {
            let (socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut stream = BufReader::new(socket);
            for _ in 0..2 {
                let mut line = String::new();
                stream.read_line(&mut line).unwrap();
                assert_eq!(line, "POST /rerank HTTP/1.1\r\n");
                let mut length = None;
                loop {
                    line.clear();
                    assert!(stream.read_line(&mut line).unwrap() > 0);
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.strip_prefix("content-length:") {
                        length = Some(value.trim().parse().unwrap());
                    }
                }
                stream.read_exact(&mut vec![0; length.unwrap()]).unwrap();
                let body = r#"{"results":[{"index":0,"relevance_score":0.75}]}"#;
                write!(
                    stream.get_mut(),
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        for _ in 0..2 {
            assert_eq!(
                reranker.score("query", &["document".into()]).unwrap(),
                [0.75]
            );
        }
        server.join().unwrap();
    }
}

#[derive(Deserialize)]
struct Response {
    results: Vec<Score>,
}
#[derive(Deserialize)]
struct Score {
    index: usize,
    relevance_score: f64,
}

impl Reranker for HttpReranker {
    fn score(&self, query: &str, documents: &[String]) -> Result<Vec<f64>> {
        let execute = || -> anyhow::Result<Vec<f64>> {
            let mut request = self.client.post(&self.endpoint).json(
                &serde_json::json!({"model": self.model, "query": query, "documents": documents}),
            );
            if let Some(token) = &self.token {
                request = request.bearer_auth(token);
            }
            let response = request.send()?.error_for_status()?;
            use std::io::Read;
            let mut bytes = Vec::new();
            response.take(1_048_577).read_to_end(&mut bytes)?;
            anyhow::ensure!(bytes.len() <= 1_048_576, "Reranker response exceeds limit");
            let response: Response = serde_json::from_slice(&bytes)?;
            let mut scores = vec![f64::NAN; documents.len()];
            for item in response.results {
                anyhow::ensure!(
                    item.index < scores.len()
                        && scores[item.index].is_nan()
                        && item.relevance_score.is_finite(),
                    "Invalid reranker index or score"
                );
                scores[item.index] = item.relevance_score;
            }
            anyhow::ensure!(
                scores.iter().all(|value| value.is_finite()),
                "Missing reranker scores"
            );
            Ok(scores)
        };
        execute().map_err(|error| MemoryError::Embedding(error.to_string()))
    }
}
