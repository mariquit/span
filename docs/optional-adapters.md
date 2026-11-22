# Optional adapters

Current policy: Git snapshot/rebuild commands, export/import MCP tools, profile
tools, and the browser viewer are disabled. Their future design is tracked in
`ROADMAP.md`. The Git and viewer sections below describe the earlier implementation,
not currently enabled entry points. See `capture-and-maintenance.md` for active
capture and HTTP behavior.

The default storage remains SQLite. These adapters are opt-in and do not change
the single-user authorization model.

## External reranker

Set `SPAN_RERANK_URL` to a compatible HTTPS endpoint, or a loopback HTTP
endpoint. Set `SPAN_RERANK_MODEL` and optionally
`SPAN_RERANK_TOKEN`. Requests contain the query and candidate overviews.
Enabling this adapter sends memory text to the configured service.

The endpoint accepts `model`, `query`, and `documents`, and returns
`results` containing `index` and `relevance_score`. Requests time out after five
seconds. Failed or malformed responses preserve the deterministic ranking.

## Experimental HNSW

Build with `cargo build -p span --features ann`, then set
`SPAN_VECTOR_INDEX=hnsw` or pass `--vector-index hnsw`. The CLI flag takes
precedence over the environment. Without either, the index type is `cosine` and
vector ranking is exact. Unknown index names fail rather than silently changing
behavior. Requesting HNSW without an `ann` build fails with an actionable error.
Eligible snapshots with more than 100 compatible embeddings use
HNSW for vector candidate selection. Lexical retrieval remains available.

The index is rebuilt per query to preserve scope, validity, and revision
eligibility. This is experimental and is not a persistent-index performance
optimization. Benchmarking and a generation-aware reusable index remain future work.

## Explicit capture hook

Pipe one `StoreInput` JSON object into `span capture`. Input is limited to
1 MiB and uses the same validation and sanitation as MCP ingestion. Configure
individual harness hooks explicitly. The command does not install hooks or
automatically collect transcripts.

## Read-only viewer

Start `span serve-http` with `SPAN_HTTP_TOKEN` configured and visit
the loopback server in a browser. The public HTML shell contains no memories.
Search requests require the token. Credentials are not persisted by the viewer.

## Git snapshots and rebuilding

`span git-snapshot SCOPE DIRECTORY` requires an existing clean Git repository
root. It writes a new snapshot directory and commits only that directory. It does
not push changes or rewrite history. Git commit hooks configured in that repository
run normally. Failed Git commands leave the snapshot available for recovery.

Markdown projections contain versioned JSON metadata and an editable Markdown
body. Importing edited bodies regenerates the abstract and overview.

`span rebuild-index SCOPE DIRECTORY OUTPUT` imports a projection directory
into a new database. Existing output files are rejected. A failed import leaves a
partial database for inspection; it must not be treated as a completed rebuild.

This provides an explicit file-to-index workflow, not automatic bidirectional
Git-primary synchronization. Links, historical revisions, evidence relationships,
and handoffs are not included in projection exports. Use SQLite backup for a
complete database backup.
