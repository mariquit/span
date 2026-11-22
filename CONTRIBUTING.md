# Contributing to Span

Span provides local, persistent memory for OpenCode and other coding harnesses. Contributions
should preserve its single-user, local-first design, explicit scope boundaries,
and deliberate handling of potentially confidential memory data.

## 1. Community expectations

Treat contributors and maintainers respectfully. Explain disagreements with
concrete examples, reproducible results, and constructive feedback.
See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) for community standards and private
reporting instructions.

## 2. Project architecture

- **`span-core/`** contains SQLite migrations, memory lifecycle operations,
  embeddings, search/ranking, sanitation, evidence tracking, and maintenance.
- **`span/`** exposes MCP Streamable HTTP and a JSON API over authenticated loopback HTTP, harness
  setup, and operational CLI commands.
- **`scripts/`** contains the opt-in capture hook, Python tests, MCP smoke test,
  and reproducible reranker benchmark.
- **`benchmarks/`** contains synthetic benchmark results and methodology.
- **`docs/`** contains context, capture, distribution, and operational guidance.

Read [README.md](README.md) for supported behavior and [WORKFLOW.md](WORKFLOW.md)
for how clients use the daemon. Discuss changes to deferred integrations or
disabled features with the maintainer before implementing them.

## 3. Development setup

### Prerequisites

- Rust 1.88 or newer, with `rustfmt` and `clippy`. Use the current stable toolchain
  for development and check compatibility with the declared minimum Rust version.
- Git for source control.
- Python 3 for the capture-hook tests and dependency-free MCP smoke test.

```sh
git clone https://github.com/mariquit/span.git
cd span
cargo build --workspace
bash scripts/install-hooks.sh
```

Default builds include FastEmbed support. A normal server launch may download the
pinned embedding model. Use the hash embedder and an isolated database for local
smoke tests without model downloads:

```sh
SPAN_EMBEDDER=hash SPAN_DB=/path/to/disposable/memory.db \
  cargo run -p span -- serve
```

Do not point development commands at your real memory database. The hash embedder
is a deterministic test fallback, not a replacement for semantic embeddings.

## 4. Guidelines and safety requirements

### Protect memory data

- Use synthetic fixtures. Never commit real memory databases, backups, capture
  queues, credentials, transcripts, confidential content, or personal information.
- Review diffs and new files before staging. Ignore patterns do not protect files
  already tracked by Git, and redaction is not a guarantee that data is safe to share.
- Keep capture opt-in. Do not add silent transcript collection or context injection.
- Keep remote inference optional and disclose what text leaves the local machine.
- Do not re-enable Git memory exports, browser UI, or shared-user functionality
  without a reviewed design and explicit agreement on the scope.

### Preserve storage and retrieval contracts

- Add schema migrations rather than assuming every database is newly created.
- Preserve provenance, supersession history, explicit scopes, and validity filtering.
- Apply privacy and eligibility rules consistently across retrieval channels.
- Keep derived memories and their evidence consistent, including on failure.
- Never introduce automatic destructive maintenance. Use previews and explicit
  application for operations that change memory state.
- Treat retrieved memory as untrusted evidence, not executable instructions.

### Maintain supported interfaces

- Write logs to stderr. Keep stdout available for machine-readable CLI output.
- Preserve authenticated loopback operation. Scope labels and private filtering
  are not multi-user authorization.
- Keep exact cosine retrieval as the default. Experimental dependencies and external
  services must remain opt-in, with clear errors or documented fallback behavior.
- Keep changes focused and consistent with surrounding Rust code. Update tool
  schemas, CLI help, tests, and documentation when public behavior changes.

## 5. Testing and quality checks

Add a regression test for bug fixes and behavior changes. Cover failure paths,
scope isolation, privacy, historical visibility, migrations, and concurrency where
relevant. Ideally demonstrate failure before the fix and success afterward.

`cargo test` runs real inference with both the default MiniLM model and the
opt-in Bekko model. On a cold cache, the tests download the pinned model files
from Hugging Face into Span's usual model cache. Set `SPAN_MODEL_CACHE` to use
another cache. If the model files are unavailable and you cannot download them,
run `cargo test -- --skip embedding_model_` to run the remaining tests without
either model inference check.

Enable the repository-local hooks with `bash scripts/install-hooks.sh`. The installer
preserves Tokensave's recognized global forwarding hooks by installing repository
hooks in the Git directory. Other existing configurations require manual integration.
Hooks are not enabled automatically
on clone and can be bypassed; contributors remain responsible for verification.

- **Pre-commit:** formatting and a locked workspace compilation check.
- **Pre-push:** full verification, including strict Clippy, default/all-feature/core-only
  tests, a release build, capture-hook tests, and isolated CLI/MCP smoke tests.
- **Pull requests:** run full verification locally and include the command, environment,
  and result in the PR description. Maintainers can run the same command before merging.
- **Releases:** tagged builds and publication run remotely in GitHub Actions. There is
  no automatic push/PR quality-check workflow. Remove obsolete required CI checks from
  repository branch-protection settings if they were configured.

The hooks and reviewers share these commands:

```sh
bash scripts/verify.sh fast
bash scripts/verify.sh full
```

For optional-feature changes, also test builds without those features. For ranking
or performance changes, document the fixture, model revision, hardware, metrics,
and limitations. Do not claim improvements from an unverified estimate. The BGE
benchmark requires separately installed Python dependencies and model files; see
[its methodology](benchmarks/bge-reranker-v2-m3-onnx.md).

## 6. Issues and pull requests

### Reporting bugs

Check [existing issues](https://github.com/mariquit/span/issues) first.
Include your operating system, Rust and binary versions, enabled features,
transport, embedder/index configuration, and a minimal synthetic reproduction.
Remove tokens, private paths, memory content, and other sensitive information from
logs before sharing them. Do not attach your memory database.

Report suspected vulnerabilities privately as described in [SECURITY.md](SECURITY.md),
not in the public issue tracker.

### Submitting changes

1. Create a focused branch from `main`.
2. Discuss large dependencies, migrations, or security-boundary changes before
   committing to a broad implementation.
3. Include tests and update documentation for changed behavior.
4. Update [CHANGELOG.md](CHANGELOG.md) for user-visible changes.
5. Run the checks above and report any checks you could not run.
6. Open a pull request explaining the problem, solution, compatibility impact,
   and verification results. Keep unrelated cleanup out of the change.

Use Conventional Commits, such as `fix(storage): preserve revision history` or
`docs(mcp): clarify daemon setup`. Explain why the change is needed in the body
when the subject alone does not make it clear. Sign off each commit as described
below.

## 7. License

By contributing, you agree that your contributions are licensed under the
project's [Apache License, Version 2.0](LICENSE). There is no contributor license
agreement (CLA); you retain copyright to your work.

### Sign your work

Sign off each commit under the [Developer Certificate of Origin 1.1](DCO). Use
your real name and a working email address:

```sh
git commit -s
git log -1 --format=%B
```

Confirm the commit message contains `Signed-off-by: Your Name <you@example.com>`.
The sign-off certifies that you have the right to submit the contribution. It is
not a cryptographic signature.

### Attribution and prior art

Preserve applicable notices and mark modified files as required by Apache-2.0 §4.
Do not copy proprietary, unlicensed, or license-incompatible material. Identify
adapted work and its license in the pull request and record applicable attribution
in [NOTICE](NOTICE) or [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md).
See [TRADEMARK.md](TRADEMARK.md) for naming and attribution guidance.
