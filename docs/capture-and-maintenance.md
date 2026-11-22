# Capture, maintenance, and loopback service

## Automatic capture

`scripts/capture_hook.py` is an opt-in successful post-tool hook adapter. Configure
the harness to invoke it after successful edit/write/patch tools. Set
`SPAN_CAPTURE=1` and the same `SPAN_HTTP_TOKEN` used by `span serve`. If the daemon
uses a non-default port, set `SPAN_HTTP_PORT` to match. The hook posts to the
daemon's `/capture-event` route and does not start its own Span process.

The adapter accepts `tool_name` or `tool`, `session_id`, `tool_use_id` or `call_id`,
and optional `cwd`. It stores a minimal activity fact, never arguments, tool output,
or transcript content. Unsupported tools and explicit failures are skipped. Missing
event identities are errors. Configure it only on successful post-tool events.

Pending sanitized events are stored in a mode-0700 local queue with mode-0600 files.
Retries reuse stable identifiers. Queue files are published atomically, and one
hook invocation drains the queue at a time. Other invocations queue their events
without waiting. Each drain attempts up to 100 files within five seconds. If the
daemon is unreachable or rejects the token, draining stops and the remaining
events stay queued.
Malformed files are moved to `quarantine/` inside the queue; ingestion failures
remain pending for retry. A bad file does not stop later events from being
attempted, but the hook reports the error and exits nonzero. Protect the queue
and its quarantine as confidential local data.

`span capture-event`, `span_capture_event`, and `/capture-event` accept
approved durable events directly. Event types are `task_complete`, `decision`, and
`fix_verified`. A caller must provide a stable event ID and `successful=true`.
IDs are unique within a scope. Distinct IDs can store identical text. Retrying
an ID does not recreate an event whose memory was forgotten.
These do not prove tool success independently; the harness supplies that signal.

## Maintenance

`span_maintenance` previews scoped potential contradictions, expired records,
and stale derived records. `apply=true` writes contradiction flags and refreshes
stale saved-query pages using a preview-then-apply evidence snapshot. It never
deletes memories. Invoke it from a trusted local scheduler if periodic maintenance
is desired. There is no implicit background data-deletion job.

`limit` is clamped to 1–100. Contradiction detection compares only the most
recently updated eligible memories up to that limit, not the entire scope.
Maintenance applies the limit to stale derived records and to stale saved-query
pages separately, after checking evidence freshness.

Knowledge-page apply requires `expected_source_ids` from the preview. Changed
evidence rejects the update. Derived records, evidence, and saved-query metadata
are written in one transaction. Preview and apply include complete cited sections
in ranked order, stopping before the next section would exceed the 100,000-byte
storage limit. Only included sections count as evidence. An empty evidence set
preserves the previous page.
A refresh that leaves the page content and evidence unchanged still updates its
saved query. Context packets and reflection omit derived records with stale
evidence at the requested time. Inspect preserved pages with `span_page_status`.
`span_page_status` exposes saved queries and stale-source status.

Reflection returns extractive cited claims and potential conflicts. It is not an
LLM and does not establish arbitrary semantic truth. Consolidation groups related
sources using shared entities or token overlap and keeps obvious opposite-polarity
claims out of the same group. These conservative heuristics still require review.

Compaction preserves every nonempty line and removes adjacent duplicate lines. It
creates a revision rather than deleting the original, and preserves the memory
type, evidence links, and knowledge page identity. Retention reports expiry;
it does not automatically discard potentially important facts.

## HTTP operation

The service binds only to `127.0.0.1` and requires a nonempty bearer token. MCP
clients connect to `/mcp` using Streamable HTTP. Ordinary tool calls return JSON;
the transport can use server-sent events for streamed responses. It uses
eight bounded concurrent operations, a 120-request-per-minute authenticated global
limit, a 1 MiB body cap, a ten-second body deadline, a thirty-second operation
deadline for JSON operations, and graceful Ctrl-C or SIGTERM shutdown. Browser
requests carrying an `Origin` header are rejected.
Logs contain response status and elapsed time, not request bodies or credentials.

A timed-out blocking mutation may finish after the response deadline. Inspect its
state before retrying; use stable capture event IDs for ingestion retries. The
service remains single-user. Internet exposure, multi-user permissions, and TLS
termination are outside the supported deployment model.

Git snapshot/rebuild commands, browser assets, export/import tools, and profile
tools are disabled pending the low-priority work in `ROADMAP.md`.
