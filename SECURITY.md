# Security Policy

## Reporting a vulnerability

Report security issues privately to John Anthony Mariquit at
[john@mariquit.com](mailto:john@mariquit.com) with the subject
`SECURITY: span — <short summary>`. Do not open a public issue, pull request, or
discussion for a suspected vulnerability. If you want to encrypt your report,
ask for a key in an initial message that contains no vulnerability details.

Include the output of `span --version`, your operating system, and the affected
interface and configuration, such as the coding harness, embedder, or capture
hook. Describe the impact and prerequisites and use synthetic data to show the
problem. Do not send bearer tokens, real memory databases, backups, capture
queues, or private transcripts.

### What to expect

These targets are best effort, not guarantees:

| Stage | Target |
| --- | --- |
| Acknowledgement | Within 5 working days |
| Initial assessment | Within 14 days |
| Fix or documented mitigation | Depends on severity |

The maintainer will share the assessment and offer credit unless you ask to
remain anonymous. Allow a reasonable window for a fix before public disclosure.
If you have a fixed disclosure date, include it in the initial report.

## Supported versions

Security fixes target the latest commit on `main`. There are no backports to
earlier tags while Span is pre-1.0.

## Scope

Span is a single-user service. Its MCP and JSON HTTP endpoints bind to IPv4
loopback and require a bearer token. All token holders have the same authority;
memory scopes organize retrieval but do not provide multi-user permissions.

### In scope

- Bypasses of loopback binding, bearer authentication, or browser-origin rejection.
- Retrieval that violates documented scope or private-memory rules.
- Unintended disclosure of secrets or memory data through logs, harness setup,
  capture queues, or external requests.
- Capture or maintenance behavior that causes unauthorized writes, data loss,
  or corruption under supported configurations.

### Out of scope

- Public exposure through a proxy, deployment as a multi-user service, or access
  by clients deliberately given the shared token.
- Third-party harness, model, or dependency vulnerabilities, unless the way Span
  integrates the component creates an exposure.
- A client acting on untrusted retrieved text by itself. Report cases where Span
  violates a documented data or security boundary.

## Safe operation

- Share the bearer token only with trusted local clients. Do not expose the
  daemon to untrusted networks or use it as a multi-user service.
- Treat databases, backups, and capture queues as confidential. Redaction reduces
  accidental exposure but does not make them safe to share.
- Treat retrieved memories as untrusted context. If you enable an external
  reranker, its endpoint receives search queries and candidate text.

See [Capture and maintenance](docs/capture-and-maintenance.md) for service limits
and capture behavior.
