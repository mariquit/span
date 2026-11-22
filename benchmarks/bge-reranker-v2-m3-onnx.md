# BGE ONNX reranker benchmark

## Scope

This is an exploratory local benchmark, not a production relevance evaluation.
The fixture contains 18 synthetic English memory records and 12 queries with one
designated relevant record each. No real user memories were read or exported.

The baseline uses the actual memory service with FastEmbed MiniLM embeddings,
exact cosine retrieval, FTS5, fusion, weighting, and MMR. The adapter arm runs the
same service with its HTTP reranker connected to local ONNX inference. Every
adapter query was verified to include the `reranker` match marker.

A separate raw-BGE arm orders all fixture records directly by model score. It is
not equivalent to the adapter's bounded relevance boost followed by MMR.

## Model and environment

- Model: `onnx-community/bge-reranker-v2-m3-ONNX`.
- Revision: `6f5ff65298512715a1e669753bc754d2bc8f367b`.
- Artifact: `onnx/model_quantized.onnx`, INT8.
- ONNX Runtime CPU provider, eight intra-operation threads, maximum 256 tokens.
- Host CPU: AMD RYZEN AI MAX+ 395, 32 logical CPUs.
- Warm model; one measured pass. Latencies are not repeated-trial confidence estimates.
- Service binary: debug build. Baseline and adapter numbers include local HTTP.

## Observed results

| Metric | Default ranking | BGE through current adapter | Raw BGE ordering |
| --- | ---: | ---: | ---: |
| MRR@10 | 0.8333 | 0.8333 | 0.7708 |
| nDCG@10 | 0.8552 | 0.8552 | 0.8077 |
| Hit@1 | 75.0% | 75.0% | 66.7% |
| Recall@5 | 91.7% | 91.7% | 91.7% |
| Median latency | 13.21 ms | 236.83 ms | 218.84 ms |
| p95 latency | 23.72 ms | 260.75 ms | 238.90 ms |

The current adapter produced no measured aggregate relevance improvement on this
fixture and added substantial CPU latency. Raw BGE ordering was worse on these
labels. Keep the default ranking enabled. These observations do not establish
that BGE is worse on other corpora, languages, precisions, or ranking policies.

An initial diagnostic run exposed that HTTP startup was not applying reranker
configuration. That wiring was fixed, and the benchmark now refuses to report
adapter results unless the reranker was actually applied. Only the corrected run
is retained in the result JSON.

## Reproduction

Download the pinned tokenizer/config files and `onnx/model_quantized.onnx` with
`hf download`. Install `numpy`, `tokenizers`, and `onnxruntime` in an isolated
Python environment, then run:

```sh
cargo build --workspace --all-features
python scripts/benchmark_reranker.py \
  --model-dir /path/to/bge-reranker-v2-m3-onnx \
  --output /tmp/bge-reranker-benchmark.json
```

The script starts temporary local services and a local ONNX endpoint. It records
per-query ranks, latencies, runtime metadata, and the fixture hash. Full results
are in `bge-reranker-v2-m3-onnx.json`. The model weights are not tracked in Git.
