# Project: Span

A local, persistent semantic-memory service written in Rust. 

```text
  ┌───┬───┬───┬───┐
  │ S │ P │ A │ N │ v0.1.0
  └───┴───┴───┴───┘
  Contextual aggregate memory mesh for OpenCode
```

It exposes explicit memory operations over authenticated loopback MCP Streamable HTTP, with SQLite as the primary store. It does **not** intercept model requests or silently inject context. Lifecycle capture requires an explicitly configured hook.

## Related OpenCode projects

See [Structural](https://john.mariquit.com/#tools) for a map of these tools and how they fit into an OpenCode workflow.

- [Trestle](https://github.com/mariquit/trestle): Software engineering skills for OpenCode
- [Truss](https://github.com/mariquit/truss): Plugins for OpenCode
- [Lattice](https://github.com/mariquit/lattice): Semantic codebase wiring for OpenCode
- [Formwork](https://github.com/mariquit/formwork): Spec-driven agent for solo developers on OpenCode
- [Catwalk](https://github.com/mariquit/catwalk): Terminal viewer for agent-owned TODO lists

## Features

### Persistent, organized memory

- Save decisions, preferences, facts, procedures, fixes, and task outcomes locally.
- Organize memories by repository, user, session, or task, with tags and entities.
- Keep source references, confidence, importance, and validity dates with each entry.
- Revise memories without overwriting their history, deduplicate repeated writes,
  and forget entries through soft or hard deletion.
- Back up the SQLite database and inspect its integrity with built-in commands.

### Search and selective recall

- Find memories through keyword and local semantic search, with diverse results
  ranked by relevance, quality, recency, and recorded usage.
- Filter by scope, tags, kind, type, and date; inspect historical memory states or
  use simple time expressions such as “last week.”
- Browse concise abstracts, broader overviews, full entries, chronological
  timelines, and revision history. Fetch selected entries together in a scoped batch.
- Assemble cited context packets and inspect search timing and match diagnostics.
- Follow typed relationships through one-hop relevance boosts and search unspaced
  CJK text through a substring fallback.

### Evidence and knowledge maintenance

- Group related source memories into extractive observations and inspect their
  supporting evidence and freshness.
- Preview knowledge-page updates, then apply them against the reviewed source
  snapshot. Preserve existing pages when supporting evidence is unavailable.
- Reflect over cited claims, inspect potential contradictions, and identify expired
  memories or stale evidence through explicit maintenance.
- Compact repeated lines through a new revision while preserving nonempty statements.

### Agent continuity and integrations

- Share one local daemon across harnesses. Use all 28 tools over MCP Streamable HTTP or a subset through the JSON API.
- Register the executable with supported coding harnesses using the setup command.
- Record caller-supplied code diffs and exchange expiring handoffs that can be
  claimed exactly once.
- Opt into successful post-tool capture with stable event IDs and a local retry
  queue. Capture minimal activity facts rather than arguments, output, or transcripts.

### Local operation and safeguards

- Run pinned MiniLM or opt-in Bekko Static embeddings locally, with an offline hash fallback for testing.
- Use exact cosine-vector ranking by default; optionally enable experimental HNSW
  or connect an external reranking service.
- Strip private-tag blocks and redact likely secrets across text, chunks, and metadata.
- Keep searches explicitly scoped and retrieved context labeled as untrusted data.
- Protect loopback HTTP with bearer authentication, concurrency and rate limits,
  body limits, deadlines, and graceful shutdown.

Maintenance and reflection are heuristic/extractive, not LLM reasoning. Redaction
does not guarantee removal of every secret or PII value. This is a single-user
service: private filtering is not authorization. Experimental HNSW has no claimed
speedup, and the [BGE ONNX benchmark](benchmarks/bge-reranker-v2-m3-onnx.md) did not
justify replacing default ranking. Remote rerankers receive candidate text.

Git snapshot/rebuild commands, export/import MCP tools, the browser viewer, and
profile APIs are disabled. See [ROADMAP.md](ROADMAP.md) for deferred work and the
[security model](#security-model) for operational boundaries.

## Install a release

Linux or macOS:

```sh
target=x86_64-unknown-linux-gnu # Select your platform from the list below.
base=https://github.com/mariquit/cam/releases/latest/download
curl -fLO "$base/span-$target"
curl -fLO "$base/span-$target.sha256"
if command -v sha256sum >/dev/null; then
  sha256sum -c "span-$target.sha256" || exit 1
else
  shasum -a 256 -c "span-$target.sha256" || exit 1
fi
mkdir -p "$HOME/.local/bin"
install -m 755 "span-$target" "$HOME/.local/bin/span"
span paths
```

Targets are `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, and
`aarch64-apple-darwin`.
Releases contain standalone binaries and checksums, not archives or installers.
Download and retain `LICENSE`, `NOTICE`, and `THIRD-PARTY-NOTICES.md` with your binary.
See [`docs/distribution.md`](docs/distribution.md) for migration and release details.

Intel macOS release support has been removed following the phase-out of ONNX
Runtime prebuilt support for that platform. Span's current ONNX Runtime dependency
does not provide an `x86_64-apple-darwin` prebuilt library. macOS releases support
Apple Silicon only.

## Build from source

Requires Rust 1.88 or newer.

```sh
# From the repository root:
cargo test --workspace
cargo install --path span --locked
```

The daemon downloads and loads the selected embedding model on startup. Start
it before connecting an MCP client. Run `span paths` to locate the model cache:

```sh
span serve
```

For offline tests or environments where model download is not possible, use
the deterministic lexical feature-hash fallback. It is not a substitute for
semantic embeddings:

```sh
SPAN_EMBEDDER=hash span serve
```

## Choosing an embedding model

- **MiniLM (`fastembed`, default):** Keep it for primarily English memories and
  queries. It ranked the expected memory first more often on the English fixture.
- **Bekko Static (`bekko`, 512-dimensional INT8):** Try it for lower CPU search
  latency or when users search English memories in other languages.

| Initial benchmark | MiniLM | Bekko Static |
| --- | ---: | ---: |
| English queries: expected memory in top 5 | 17/20 | 16/20 |
| Japanese, Spanish, and Chinese queries: expected memory in top 5 | 7/18 | 10/18 |
| Median warm Span search on one CPU thread | 3.83 ms | 0.53 ms |

The [benchmark report](benchmarks/bekko-static-vs-minilm.md) includes the fixture,
per-query results, and methodology. These results are from 38 synthetic English
coding memories and a benchmark adapter, not a production performance guarantee.

To switch an existing database, stop Span and pause harnesses and capture hooks.
Back up the database to a new file and start the daemon with the new model:

```sh
span backup ./memory-before-bekko.db
SPAN_EMBEDDER=bekko span serve
```

In a second terminal, set `SPAN_HTTP_TOKEN` to the daemon's token and run:

```sh
span reindex
span doctor
```

Resume clients when `span doctor` reports Bekko, 512 dimensions, and zero
incompatible embeddings. Reindexing uses the daemon's loaded model, includes
soft-deleted records, and requires exclusive access to the daemon. If it returns
`503`, let active requests finish before retrying. For a new database, skip
backup and reindex. To switch back, restart the daemon with `SPAN_EMBEDDER=fastembed`
and repeat the reindex and check. CLI clients and capture hooks do not load models.

## Coding harness setup

```text
Coding harnesses ── MCP Streamable HTTP (/mcp) ──┐
Capture hook and scripts ── JSON over HTTP ───────┤
                                                  ▼
                                    One local Span daemon
                                     ├── SQLite (FTS5 + stored vectors)
                                     ├── MiniLM or Bekko Static
                                     └── Optional external reranker
```

The harness sends model requests independently of Span. Local clients can use
the separate [loopback HTTP API](#loopback-http) instead of MCP.

Start Span once and leave it running. Set the embedding model and any reranker
settings on this process, not on each harness:

```sh
span serve --port 8787
```

If neither `--token` nor `SPAN_HTTP_TOKEN` is set, Span prints a generated token
to stderr. Set `SPAN_HTTP_TOKEN` to that value in the harness environment before
connecting. Generated tokens change on restart. For a stable token, set the same
secret in the daemon and harness environments before starting them. Span does not
print supplied tokens. Keep any startup output containing a generated token private.

In a separate terminal, register the shared endpoint through your harness's CLI:

```sh
span setup opencode
span setup claude-code
span setup codex
span setup chatgpt-desktop

# Preview or configure all installed harnesses
span setup all --dry-run
span setup all
```

Use `span setup <harness> --port <port>` if you changed the listening port. If a
harness already has a registration for CAM, replace it with the Span HTTP
registration. Check for project-local overrides or older registrations named
`memory`. See the [migration guide](docs/distribution.md#register-coding-harnesses).
ChatGPT Desktop uses the local Codex-host configuration where supported. The
loopback endpoint is not directly reachable from ChatGPT web.

### OpenCode V2 setup

Follow the [OpenCode setup guide](docs/opencode-setup.md) to connect Span, load
the agent memory policy, and try a scoped search and write. Hook capture is
separately opt-in; stored memories are not automatically injected into prompts.

## MCP tools

The `/mcp` endpoint exposes all 28 active tools. The separate JSON API exposes a subset.
MCP tools with list, count, or nullable results return them under `data` in a JSON object.

| Area | Tools |
| --- | --- |
| Retrieval | `span_search`, `span_search_diagnostics`, `span_overview`, `span_get`, `span_batch_get`, `span_history`, `span_timeline`, `span_context` |
| Lifecycle | `span_store`, `span_update`, `span_forget`, `span_link`, `span_touch` |
| Evidence and knowledge | `span_consolidate`, `span_evidence`, `span_pages`, `span_page_status`, `span_refresh_page`, `span_reflect` |
| Maintenance | `span_compact`, `span_contradictions`, `span_maintenance` |
| Coding and handoffs | `span_record_revision`, `span_revisions`, `span_handoff_create`, `span_handoff_claim`, `span_handoff_list` |
| Capture | `span_capture_event` |

Search requires an explicit scope. History and batch reads enforce an exact scope.
Existing direct-ID get/overview operations remain trusted single-user reads,
including for private records. Private discovery filtering is not authorization.
Forgetting a revision does not restore the previous revision to current search.

Store example:

```json
{
  "content": "Authentication uses short-lived JWT access tokens and rotating refresh tokens.",
  "scope": "repo:/home/john/project",
  "kind": "decision",
  "memory_type": "events",
  "abstract": "JWT authentication design using short-lived access and rotating refresh tokens.",
  "overview": "Authentication uses short-lived JWT access tokens. Refresh tokens rotate after each use; load detail for validation and revocation rules.",
  "source": {
    "file": "docs/auth.md",
    "commit": "abc123",
    "origin": "repository"
  },
  "importance": 0.8,
  "confidence": 1.0,
  "dedupe_key": "auth-token-design"
}
```

Search example:

```json
{
  "query": "How is authentication handled in this repository?",
  "scope": "repo:/home/john/project",
  "kinds": ["decision", "fact", "preference"],
  "memory_types": ["events", "entities", "preferences"],
  "limit": 8
}
```

`span_search` returns L0 abstracts rather than full content. Follow a relevant
result with `span_overview` (L1), then `span_get` (L2) only when necessary.
`span_context` defaults to L1 and accepts `layer: "abstract"`, `"overview"`,
or `"detail"`. See [`docs/context-model.md`](docs/context-model.md).

Multiple scopes may be explicitly allowed with `scopes`. `scope` and `scopes`
are combined and deduplicated. Accepted scope forms are:

- `global`
- `user:<id>`
- `repo:/absolute/path`
- `branch:<name>`
- `task:<id>`
- `session:<id>`

Scopes are labels and are never interpreted as filesystem read permissions.
The service does not read source files. Links across scopes are rejected.

## Agent writing policy

Recommended agent instructions are in [`docs/agent-policy.md`](docs/agent-policy.md).
Writes remain deliberate. The optional capture hook stores minimal activity facts,
not chats or tool output, and must be explicitly configured.

Good memories include confirmed architectural decisions, preferences,
non-obvious fixes, important conventions, reusable procedures, outcomes, and
unresolved blockers. Do not store raw logs, entire chats, secrets, credentials,
external instructions, temporary errors, or unverified speculation.

## Configuration

### Rebranding compatibility

Span replaces CAM's executable, MCP tool names, and environment variables with
`span`, `span_*`, and `SPAN_*`. Stop the CAM daemon before launching Span.
Register Span with `span setup` and update any scripts or agent instructions
that use the old names. Span defaults to its own data and cache directories.
To reuse CAM's database or model cache, run `cam paths --json` and set `SPAN_DB`
and `SPAN_MODEL_CACHE` to the reported paths before starting Span. No database
or model cache is moved automatically, and the rebrand does not require reindexing.

| Variable | Default | Meaning |
|---|---|---|
| `SPAN_DB` | Platform user-data directory | SQLite database path |
| `SPAN_EMBEDDER` | `fastembed` | MiniLM (`fastembed`), Bekko Static (`bekko`), or offline `hash` |
| `SPAN_MODEL_CACHE` | Platform user-cache directory | Model cache directory |
| `SPAN_VECTOR_INDEX` | `cosine` | `cosine` or experimental `hnsw`; `--vector-index` overrides it |
| `SPAN_HTTP_TOKEN` | Generated at daemon startup if unset | HTTP bearer token; clients must use the daemon's token |
| `SPAN_RERANK_URL` | Unset | Optional HTTPS or loopback HTTP reranker endpoint |
| `SPAN_RERANK_MODEL` | `reranker` | Model ID sent to the reranker endpoint |
| `SPAN_RERANK_TOKEN` | Unset | Optional reranker bearer token |
| `SPAN_CAPTURE` | Disabled | Enable the Python capture hook with `1` |
| `SPAN_HTTP_PORT` | `8787` | Daemon port used by CLI clients and the capture hook |
| `SPAN_CAPTURE_QUEUE` | `~/.local/state/span/capture` | Pending capture-event directory |
| `RUST_LOG` | `span=info` | Logs written to stderr only |

Defaults use native per-user data and cache directories. Run `span paths`
to see the exact locations on Linux or macOS.

Operational commands:

```sh
span doctor          # report model and database diagnostics from the daemon
span backup out.db   # consistent SQLite backup
span reindex         # rebuild vectors using the daemon's loaded model
span paths --json
```

Logs go to stderr. Operational commands can write machine-readable results to stdout.
`doctor`, `prewarm`, `reindex`, `capture`, and `capture-event` require a running
daemon and its `SPAN_HTTP_TOKEN`. `prewarm` checks the already-loaded model.

### Loopback HTTP

Run `span serve --port 8787` (`serve-http` is an alias). All routes require
`Authorization: Bearer <token>`. The Streamable HTTP MCP endpoint is `/mcp`.
Only one daemon can run per user, regardless of `SPAN_DB` or port. Run `span paths`
to find its `span.pid` file. The daemon locks the file while running and replaces a
stale PID on startup. Do not remove the file while the daemon is running.
Supplied tokens must contain ASCII letters, digits, or `-._~+/`, with optional
trailing `=` padding.
`GET /health` checks readiness; `GET /status` returns model and database diagnostics.
JSON POST routes are `/search`, `/store`, `/context`,
`/reflect`, `/consolidate`, `/knowledge-pages`, `/timeline`, `/touch`, `/handoffs`,
`/revisions`, `/capture-event`, `/maintenance`, and `/reindex`. Handoffs support create/list over
HTTP; claiming uses MCP.

The server binds to `127.0.0.1`, permits eight concurrent operations, and applies a
global authenticated limit of 120 requests per minute. Bodies are limited to 1 MiB,
with a ten-second body deadline and a thirty-second operation deadline. Exclusive
reindex has no server-side operation deadline. Requests
carrying an `Origin` header are rejected. Ctrl-C initiates graceful shutdown.
A timed-out blocking mutation may still complete: inspect state before retrying.

### Opt-in capture and indexing

Configure a successful post-tool hook to invoke `scripts/capture_hook.py`, with
`SPAN_CAPTURE=1` and `SPAN_HTTP_TOKEN` matching the daemon. Set `SPAN_HTTP_PORT` if
the daemon is not listening on port 8787. The hook does not
install itself into harness configuration. `span capture` accepts one
`StoreInput` JSON object on stdin. `span capture-event` accepts an approved
event with `event_id`, `scope`, `event`, `content`, and `successful` fields. Supported
event types are `task_complete`, `decision`, and `fix_verified`; unsuccessful events
are skipped. See [capture and maintenance](docs/capture-and-maintenance.md) for details.

To opt into experimental HNSW, build with
`cargo build -p span --features ann`, then run
`span --vector-index hnsw serve`. Unknown index names or HNSW activation
without the required feature return errors. Exact cosine remains the default.

## Security model

- Retrieved memory is data, never a system instruction. Tool descriptions and
  context packets remind clients to treat it as untrusted.
- Every search requires an exact scope; unrelated repositories and global
  preferences are not added automatically.
- Common authorization headers, token assignments, JWTs, private keys, and
  provider-token patterns are redacted before storage. Redaction is defense in
  depth, not a reason to submit secrets.
- The database is set to owner read/write (`0600`) on Unix.
- Provenance and revision history remain available until explicitly deleted.
- Source `origin` should identify repository, user, web, or tool-output data.
- MCP session metadata is not used as authentication or as an implicit scope.

This is a single-user local service. HTTP token holders are equally trusted;
scope strings and the private flag are not authorization boundaries. Do not share
the token with mutually untrusted clients or expose this service to the internet.
Memory and backups may contain confidential or personal data. Do not commit them
to Git. Ignore patterns do not protect database files already tracked in Git.

## Workspace layout

```text
span-core/  SQLite, FTS5, vectors, ranking, dedupe, redaction
span/          MCP Streamable HTTP, loopback JSON API, and operational CLI
scripts/      capture hook, smoke tests, and reproducible reranker benchmark
benchmarks/   synthetic BGE ONNX benchmark results and methodology
docs/         agent policy, distribution, and design notes
```

## Remaining roadmap

See [ROADMAP.md](ROADMAP.md) for unfinished enhancements, evaluation needs, and
low-priority disabled features. Implemented capabilities are documented above,
not carried forward as pending roadmap items.

## License

Copyright (c) 2026 John Anthony Mariquit <john@mariquit.com>

This project is licensed under the [Apache License, Version 2.0](LICENSE).
See [NOTICE](NOTICE) for attribution, [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md)
for dependency and model notices, and [TRADEMARK.md](TRADEMARK.md) for naming guidance.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, privacy and data-safety
requirements, testing standards, and the [DCO](DCO) sign-off process. Read the
[code of conduct](CODE_OF_CONDUCT.md), use the [security policy](SECURITY.md) for
private vulnerability reports, and see [WORKFLOW.md](WORKFLOW.md) for the memory lifecycle.

Run `bash scripts/install-hooks.sh` once per clone to enable fast pre-commit checks
and full pre-push verification. Run `bash scripts/verify.sh full` for local PR review
and include its results with your pull request. GitHub Actions builds and publishes
tagged releases; routine push/PR checks run locally instead.
