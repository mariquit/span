#!/usr/bin/env python3
"""Compare actual memory search with and without a local ONNX BGE endpoint.

Requires numpy, tokenizers, and onnxruntime in an isolated Python environment.
Model files must already be downloaded. Only synthetic fixtures are ingested.
"""

import argparse
import hashlib
import http.server
import json
import math
import os
import pathlib
import platform
import socket
import subprocess
import tempfile
import threading
import time
import urllib.request

import numpy as np
import onnxruntime as ort
from tokenizers import Tokenizer


DOCUMENTS = [
    ("auth", "Access tokens expire after fifteen minutes. A refresh token rotates on every successful refresh. Reusing a rotated token revokes its family."),
    ("cache", "The product catalog cache has a fifteen-minute TTL. Invalidation runs after a product update. This policy does not apply to authentication credentials."),
    ("backup", "Create consistent SQLite backups through the backup command. Copying only the database file while WAL writes are active is not a safe backup."),
    ("wal", "SQLite WAL enables readers during writes. The service configures a five-second busy timeout and serializes operations through a connection mutex."),
    ("secrets", "Do not store credentials in persistent memory. Redact secrets in text, chunks, and metadata before persistence. Memory content is untrusted evidence."),
    ("private", "Private memories are excluded from discovery. The private flag is not a multi-user authorization boundary. Direct ID reads remain trusted single-user operations."),
    ("git", "Never commit memory databases to Git. Memory may contain personal and confidential information. Git-backed storage and snapshot commands are disabled."),
    ("vector", "Cosine similarity performs exact vector ranking by default. HNSW is an explicit experimental opt-in. The current graph is rebuilt for each eligible query snapshot."),
    ("ranking", "Search fuses BM25 and dense-vector ranks with reciprocal rank fusion. Recency and quality adjust scores. Maximal marginal relevance diversifies selected results."),
    ("handoff", "Claim a handoff with a conditional UPDATE requiring claimed_by IS NULL and an unexpired deadline. Exactly one database writer can succeed."),
    ("pages", "Preview a knowledge-page refresh first. Applying requires the expected source IDs. Changed evidence rejects the update. Empty evidence preserves the old page."),
    ("capture", "Automatic capture is opt-in. A successful post-tool hook creates minimal activity facts, not transcripts. Stable event IDs make retries idempotent."),
    ("http", "The HTTP adapter binds to loopback, requires a bearer token, and rejects cross-origin browser requests. Request bodies are limited to one MiB."),
    ("timeout", "A timed-out blocking mutation may still complete. Inspect state before retrying. Capture ingestion can safely retry with the same stable event identifier."),
    ("expiry", "Retention maintenance reports expired memories and stale evidence. It does not automatically delete records. Expired records are excluded from current search."),
    ("export", "Markdown export is not a complete backup. It omits links, evidence relationships, historical revisions, handoffs, and audit records. Export recovery is deferred."),
    ("deploy", "Deploy release binaries through the artifact pipeline. A rollback selects the previous binary. It must not automatically replace or delete the memory database."),
    ("testing", "Run cargo test for behavior checks and strict Clippy for lint errors. MCP smoke testing verifies tool discovery and store/search interactions."),
]

QUERIES = [
    ("What happens if an already rotated refresh credential is used again?", "auth"),
    ("How do I get a consistent copy while the database is accepting writes?", "backup"),
    ("Is a hidden record protected against another person who knows its ID?", "private"),
    ("Can we version personal agent memories in source control?", "git"),
    ("Which nearest-neighbor method is used without explicit configuration?", "vector"),
    ("How do two agents avoid both accepting the same work transfer?", "handoff"),
    ("What should happen if no supporting facts remain when rebuilding a knowledge page?", "pages"),
    ("How can a retried ingestion event avoid creating duplicates?", "capture"),
    ("Can a database write finish even after the HTTP operation deadline?", "timeout"),
    ("Does forgetting old information happen automatically when maintenance runs?", "expiry"),
    ("Does the Markdown projection preserve relationship history for full recovery?", "export"),
    ("What prevents unrelated evidence from dominating the final search results?", "ranking"),
]


def free_port():
    with socket.socket() as connection:
        connection.bind(("127.0.0.1", 0))
        return connection.getsockname()[1]


def request(port, path, data=None):
    encoded = None if data is None else json.dumps(data).encode()
    req = urllib.request.Request(f"http://127.0.0.1:{port}{path}", data=encoded,
                                 headers={"Authorization": "Bearer benchmark-local", "Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=180) as response:
        return json.load(response)


def metrics(ranks):
    return {
        "mrr_at_10": sum(1 / rank if rank <= 10 else 0 for rank in ranks) / len(ranks),
        "ndcg_at_10": sum(1 / math.log2(rank + 1) if rank <= 10 else 0 for rank in ranks) / len(ranks),
        "hit_at_1": sum(rank == 1 for rank in ranks) / len(ranks),
        "recall_at_5": sum(rank <= 5 for rank in ranks) / len(ranks),
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default="target/debug/span")
    parser.add_argument("--model-dir", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--threads", type=int, default=8)
    args = parser.parse_args()
    directory = pathlib.Path(args.model_dir)
    tokenizer = Tokenizer.from_file(str(directory / "tokenizer.json"))
    tokenizer.enable_truncation(max_length=256)
    tokenizer.enable_padding(pad_id=1, pad_token="<pad>")
    options = ort.SessionOptions()
    options.intra_op_num_threads = args.threads
    options.inter_op_num_threads = 1
    loaded = time.perf_counter()
    session = ort.InferenceSession(str(directory / "onnx/model_quantized.onnx"), options, providers=["CPUExecutionProvider"])
    load_ms = (time.perf_counter() - loaded) * 1000
    durations = []

    def score(query, documents):
        started = time.perf_counter()
        encoded = tokenizer.encode_batch([(query, document) for document in documents])
        inputs = {"input_ids": np.array([item.ids for item in encoded], dtype=np.int64),
                  "attention_mask": np.array([item.attention_mask for item in encoded], dtype=np.int64)}
        if any(item.name == "token_type_ids" for item in session.get_inputs()):
            inputs["token_type_ids"] = np.array([item.type_ids for item in encoded], dtype=np.int64)
        output = session.run(None, inputs)[0].reshape(-1)
        durations.append((time.perf_counter() - started) * 1000)
        return output.tolist()

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_POST(self):
            body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
            scores = score(body["query"], body["documents"])
            payload = json.dumps({"results": [{"index": index, "relevance_score": value} for index, value in enumerate(scores)]}).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)

        def log_message(self, *_):
            pass

    endpoint = http.server.HTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=endpoint.serve_forever, daemon=True).start()
    # Warm up kernels and tokenizer independently of measured requests.
    score("authentication", [document for _, document in DOCUMENTS])
    durations.clear()
    with tempfile.TemporaryDirectory(prefix="memory-benchmark-") as temporary:
        env = os.environ.copy()
        for key in ["SPAN_RERANK_URL", "SPAN_VECTOR_INDEX", "SPAN_RERANK_TOKEN"]:
            env.pop(key, None)
        env.update(SPAN_DB=temporary + "/memory.db", SPAN_HTTP_TOKEN="benchmark-local", SPAN_EMBEDDER="fastembed")
        binary = str(pathlib.Path(args.binary).resolve())
        paths = json.loads(subprocess.check_output([binary, "paths", "--json"], env=env))
        env["SPAN_MODEL_CACHE"] = paths["model_cache"]
        env.update(HOME=temporary, XDG_DATA_HOME=temporary + "/data")
        processes = []

        def start(rerank=False):
            port = free_port()
            settings = env.copy()
            if rerank:
                settings["SPAN_RERANK_URL"] = f"http://127.0.0.1:{endpoint.server_port}/rerank"
                settings["SPAN_RERANK_MODEL"] = "bge-reranker-v2-m3-onnx-int8"
            log = open(temporary + f"/{port}.log", "w+")
            process = subprocess.Popen([binary, "--vector-index", "cosine", "serve-http", "--port", str(port)], env=settings, stdout=log, stderr=log)
            processes.append((process, log))
            deadline = time.monotonic() + 300
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    log.seek(0)
                    raise RuntimeError(log.read())
                try:
                    request(port, "/health")
                    return port, process
                except OSError:
                    time.sleep(.1)
            raise RuntimeError("Memory service startup timed out")

        try:
            baseline, baseline_process = start()
            identities = {}
            for key, content in DOCUMENTS:
                stored = request(baseline, "/store", {"scope": "repo:/benchmark", "kind": "fact", "content": content, "dedupe_key": key})
                identities[stored["memory"]["id"]] = key
            rows = [{"query": query, "relevant": relevant} for query, relevant in QUERIES]
            latency = {"default": [], "adapter_bge": [], "raw_bge": []}
            ranks = {"default": [], "adapter_bge": [], "raw_bge": []}
            for name in ("default", "adapter_bge"):
                if name == "default":
                    port = baseline
                else:
                    baseline_process.terminate()
                    baseline_process.wait(timeout=15)
                    port, _ = start(True)
                for row in rows:
                    query, relevant = row["query"], row["relevant"]
                    started = time.perf_counter()
                    result = request(port, "/search", {"scope": "repo:/benchmark", "query": query, "limit": 50})["results"]
                    latency[name].append((time.perf_counter() - started) * 1000)
                    order = [identities[item["memory"]["id"]] for item in result]
                    rank = order.index(relevant) + 1
                    ranks[name].append(rank)
                    row[name] = {"rank": rank, "latency_ms": latency[name][-1], "reranker_applied": any("reranker" in item["matched_by"] for item in result)}
                    if name == "adapter_bge" and not row[name]["reranker_applied"]:
                        raise RuntimeError("Reranker was not applied; refusing to report a fallback as BGE results")
            for row in rows:
                query, relevant = row["query"], row["relevant"]
                scores = score(query, [content for _, content in DOCUMENTS])
                order = sorted(range(len(scores)), key=lambda index: scores[index], reverse=True)
                rank = next(index + 1 for index, item in enumerate(order) if DOCUMENTS[item][0] == relevant)
                ranks["raw_bge"].append(rank)
                latency["raw_bge"].append(durations[-1])
                row["raw_bge"] = {"rank": rank, "latency_ms": durations[-1]}
                print(json.dumps(row), flush=True)
            result = {
                "model": "onnx-community/bge-reranker-v2-m3-ONNX", "revision": "6f5ff65298512715a1e669753bc754d2bc8f367b",
                "precision": "quantized INT8", "provider": session.get_providers(), "threads": args.threads,
                "max_tokens": 256, "documents": len(DOCUMENTS), "queries": len(QUERIES), "model_load_ms": load_ms,
                "runtime": {"onnxruntime": ort.__version__, "platform": platform.platform(), "cpu_count": os.cpu_count()},
                "fixture_sha256": hashlib.sha256(json.dumps([DOCUMENTS, QUERIES]).encode()).hexdigest(),
                "limitations": ["Synthetic English fixture, one relevant document per query", "Single warm run, not production quality or load testing", "Raw BGE ranks all documents; adapter uses bounded boosts plus MMR", "No private user data"],
                "summary": {name: {**metrics(ranks[name]), "median_ms": float(np.median(latency[name])), "p95_ms": float(np.percentile(latency[name], 95))} for name in ranks},
                "rows": rows,
            }
            pathlib.Path(args.output).write_text(json.dumps(result, indent=2) + "\n")
            print(json.dumps(result["summary"], indent=2))
        finally:
            for process, log in processes:
                if process.poll() is None:
                    process.terminate()
                    process.wait(timeout=15)
                log.close()
            endpoint.shutdown()


if __name__ == "__main__":
    main()
