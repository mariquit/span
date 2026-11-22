# Bekko Static vs MiniLM: initial Span benchmark

Bekko Static was faster and recovered more cross-language matches on this
fixture. MiniLM placed the expected memory first more often for English queries.

| Metric | MiniLM 384d | Bekko Static 512d INT8 |
| --- | ---: | ---: |
| Span hybrid top-1, all queries | 14/38 | 17/38 |
| Span hybrid top-5, all queries | 24/38 | 26/38 |
| Span hybrid top-1, English | 13/20 | 10/20 |
| Span hybrid top-5, English | 17/20 | 16/20 |
| Span hybrid top-5, Japanese / Spanish / Chinese | 7/18 | 10/18 |
| Vector-only top-5, all queries | 26/38 | 27/38 |
| Warm model encoding p50 / p95 | 3.07 / 4.39 ms | 0.039 / 0.050 ms |
| Warm Span search p50 / p95 | 3.83 / 5.01 ms | 0.53 / 0.68 ms |

## Method

- The [fixture](bekko-static-vs-minilm-fixture.json) contains 38 English coding
  memories and 38 queries: 20 English and six each in Japanese, Spanish, and
  Chinese. Each query has one labeled expected memory. The non-English sets
  ask the same six questions in translation, so they are not independent samples.
- Both models reindexed separate SQLite backups of the same database. The
  harness used Span's Rust search implementation with its normal FTS5, cosine,
  and diversity ranking and no external reranker. The vector-only comparison
  ranked the stored vectors by cosine similarity without hybrid adjustments.
- A persistent Python ONNX Runtime 1.32.0 worker encoded each model's inputs on
  one CPU thread on an AMD Ryzen AI Max+ 395. Encoding includes tokenization,
  ONNX inference, pooling when applicable, and normalization. Search includes
  Span ranking and the worker round trip, but excludes model loading and HTTP/MCP.
  One warm-up pass preceded nine timed passes (342 searches per model).
- The MiniLM artifact was `Qdrant/all-MiniLM-L6-v2-onnx/model.onnx`, revision
  `d13954661f83248295ba75c1ed411eef3b7b936e`. The adapter's query vectors had a
  minimum cosine similarity of 0.9999996 with Span's native MiniLM vectors.
  The Bekko artifact was `hotchpotch/bekko-static-embedding-v1/onnx/model_int8_dim512.onnx`,
  revision `c18167b8f244c1bc6de21ce9b710bac8a29c4615`.

The [per-query ranks](bekko-static-vs-minilm-ranks.json) use one-based ranks.
`null` means the expected memory did not appear in the top five hybrid results.
These adapter timings are not measurements of the native Bekko integration.
The small synthetic fixture does not establish production latency, quality on
non-English memories, or a general relevance improvement.

See [Choosing an embedding model](../README.md#choosing-an-embedding-model) for migration instructions.
