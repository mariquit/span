# Changelog

All notable changes to this project will be documented here.

## 0.1.0

### Added

- Initial SQLite-backed semantic memory core.
- MCP stdio tools for search, get, store, update, forget, link, and context.
- FTS5 and pinned local vector retrieval with hybrid ranking.
- Cross-platform operational CLI, harness setup, backup, doctor, and reindex.
- Progressive L0 abstract, L1 overview, and L2 detail retrieval.
- Memory taxonomy for profile, preferences, entities, events, identity, soul,
  cases, trajectories, and experiences.
- Opt-in Bekko Static 512-dimensional INT8 embeddings and a benchmark comparing
  retrieval and CPU latency with the default MiniLM model.
- Scoped batch retrieval, bounded revision history, compact timelines, access-aware
  ranking, tag/date filters, historical queries, and search diagnostics.
- Evidence-backed extractive observations, knowledge-page preview/apply refresh,
  freshness inspection, cited reflection, and explicit maintenance operations.
- Caller-supplied coding revision records and expiring, claim-once agent handoffs.
- Opt-in lifecycle capture with stable event IDs and a restricted-permission retry queue.
- Authenticated loopback HTTP with bounded concurrency, rate and request-body limits,
  deadlines, origin rejection, and graceful shutdown.
- Explicit opt-in HNSW indexing while keeping exact cosine retrieval as the default.
- Optional external reranker integration and a reproducible synthetic BGE v2 m3
  INT8 ONNX benchmark. The measured results did not justify changing default ranking.
- A contribution guide covering development, privacy, testing, and pull requests.

### Changed

- Rebranded CAM as Span: Contextual aggregate memory mesh for OpenCode. The CLI,
  MCP tools, environment variables, and release assets use the new names.
  Existing databases and embedding identities remain compatible.
- Replaced MCP stdio with a shared, authenticated loopback Streamable HTTP
  daemon. Harness registration and the automatic capture hook use the daemon.
- Changed the project license to Apache License, Version 2.0. Third-party licenses are unchanged.
- Removed Intel macOS release builds following the phase-out of compatible ONNX
  Runtime prebuilts. Apple Silicon macOS remains supported.
- Updated Linux x86-64 release builds to Ubuntu 24.04 for compatibility with the
  native ONNX Runtime dependency.

- Simplified tagged releases to one native build matrix publishing standalone
  binaries and SHA-256 checksums, with license and third-party notices attached
  separately. Removed cargo-dist installers, archives, and configuration.

- Moved routine quality checks to opt-in local pre-commit/pre-push hooks and a
  shared verification script. Pull requests report local verification results;
  GitHub Actions runs release builds only on version-tag pushes.

- Disabled Git snapshot/rebuild commands, export/import MCP tools, browser UI, and
  profile APIs pending reviewed low-priority designs.

### Fixed

- Extended sanitation to chunks and metadata, improved private-tag handling, and
  excluded private records from discovery.
- Enforced HTTP authentication and database-atomic handoff claims.
- Corrected historical retrieval and graph eligibility, removed the 2,000-record
  candidate cutoff, and batched vector loading.
- Made derived memory/evidence writes transactional and required reviewed source
  IDs when applying knowledge-page refreshes.
- Preserved nonempty statements during compaction and reduced contradiction
  false positives by requiring opposing polarity.
- Applied reranker configuration to HTTP startup and required benchmark runs to
  verify that reranking was actually used.
- Updated MCP smoke coverage and resolved strict lint failures.
