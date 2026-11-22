# Context layers and memory taxonomy

Span organizes durable memories using progressive context layers and a high-level
taxonomy, alongside explicit scopes, provenance, and hybrid retrieval.

## Progressive context layers

Every memory has three projections:

| Layer | Field | Generated budget | Use |
|---|---|---:|---|
| L0 | `abstract` | 256 characters | Vector retrieval and quick relevance checks |
| L1 | `overview` | 4,000 characters | Navigation, reranking context, and bounded context packets |
| L2 | `content` | 100,000 bytes maximum | Complete detail loaded explicitly |

Call flow:

1. `span_search` returns only L0 previews, identifiers, scores, and citations.
2. `span_overview` loads L1 for a relevant result.
3. `span_get` loads L2 and links only when full detail is needed.
4. `span_context` defaults to L1. Set `layer` to `abstract`, `overview`, or
   `detail` explicitly; the token budget still applies.

Callers may provide L0 and L1 when writing. If omitted, the core generates
deterministic bounded projections without calling an LLM. Generated text prefers
a sentence or line boundary. Secret redaction happens before layer generation.

Dense vectors contain only L0 plus whitelisted structured retrieval fields:
memory type, kind, entities, and tags. FTS5 indexes every layer, preserving exact
symbol, path, and error-message search. This keeps vector reindexing stable and
prevents full detail from being silently injected into search results.

These projections are record-level because `span` stores durable memories rather than a
virtual filesystem. Type/topic aggregate overviews can be added later without
changing the L0/L1/L2 tool contract.

## High-level memory types

`memory_type` organizes durable knowledge by purpose. `kind` remains a finer
retrieval and provenance label, so an event may be a `decision`, `task_result`,
or `conversation_summary`, for example.

| Memory type | Intended content |
|---|---|
| `profile` | Stable user background and basic information |
| `preferences` | User preferences organized by topic |
| `entities` | People, projects, organizations, systems, and other entities |
| `events` | Decisions, milestones, task results, and other event records |
| `identity` | Assistant name, persona, temperament, and introduction |
| `soul` | Assistant principles, boundaries, style, and continuity |
| `cases` | Concrete tasks with inputs, evidence, and evaluation criteria |
| `trajectories` | Reusable multi-step operation contracts and execution paths |
| `experiences` | Durable guidance distilled from execution outcomes |

When `memory_type` is omitted, it is inferred for backward compatibility:

| Existing kind | Inferred memory type |
|---|---|
| `preference` | `preferences` |
| `entity`, `fact` | `entities` |
| `procedure`, `error_fix` | `experiences` |
| `decision`, `task_result`, `conversation_summary` | `events` |

Use an explicit type when storing profile, identity, soul, case, or trajectory
memories. Search and context calls can filter by both `memory_types` and `kinds`.

## Safety and precedence

Identity and soul are memory data, not system instructions. They remain below
current user instructions, repository instructions, and repository files in
the precedence order. Retrieved content is always labeled untrusted.
