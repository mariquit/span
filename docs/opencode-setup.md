# Set up Span in OpenCode V2

Use Span to search and save durable repository decisions explicitly. This setup
loads the static [agent memory policy](agent-policy.md). It does not install a
hook or inject stored memories into prompts.

## 1. Connect Span over MCP

[Install Span](../README.md#install-a-release) and start the daemon on the same
machine as the OpenCode server:

```sh
span serve --port 8787
```

If `SPAN_HTTP_TOKEN` is not already set for the daemon, Span prints a generated
token to stderr. Set `SPAN_HTTP_TOKEN` to that value in the OpenCode server's
environment. A generated token changes on each Span restart. For a stable token,
set the same secret in both environments before starting either service. Keep the
token out of project files, prompts, and memory records.

From the project you want to use, register Span globally and check its status:

```sh
span setup opencode
opencode mcp list
```

Expect `span` to show `connected`. If you changed the daemon port, pass the same
`--port` to `span setup opencode`. If the connection fails, check the daemon, the
URL, and whether the OpenCode server has the matching token. An already-running
background service does not inherit a token you just exported in your shell. To
check with the current shell's environment, run `opencode mcp list --standalone`;
use `opencode --standalone` to open a session in that same environment.
If the CLI reports `No MCP servers configured`, check whether OpenCode loaded the
file containing `mcp.servers.span` with `opencode debug config`. Do not share
unredacted config output. You can also check the connection in OpenCode with
`/mcps`.

### Manual registration

As an alternative to `span setup opencode`, merge the following into your
existing [OpenCode configuration](https://opencode.ai/v2/docs/config). Use your
global configuration for all projects or the project's `opencode.jsonc` for one
project. Span uses a bearer header, not OAuth. If OpenCode reports `needs
authentication`, verify the token and set `oauth` to `false` rather than signing
in through OpenCode.

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "span": {
        "type": "remote",
        "url": "http://127.0.0.1:8787/mcp",
        "oauth": false,
        "headers": {"Authorization": "Bearer {env:SPAN_HTTP_TOKEN}"},
        "timeout": {
          "startup": 60000,
          "execution": 120000
        }
      }
    }
  }
}
```

Verify with `opencode mcp list` from the project. A project registration named
`span` replaces the entire global `span` entry, so include the URL and header in
any project override.

## 2. Load the agent memory policy

In the repository where you want the policy, use `docs/agent-policy.md` from this
repository as-is. If you are setting up another repository, copy the policy there
first. Add its path to the root `opencode.jsonc`, preserving any existing
`instructions` entries:

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "instructions": ["docs/agent-policy.md"]
}
```

Open an OpenCode session in that repository and ask the agent, without calling
Span, which scope it should search at the start of a repository task. It should
identify only the exact checkout scope. If it does not, check the policy path;
OpenCode skips missing [instruction files](https://opencode.ai/v2/docs/instructions).

## 3. Search, verify, and save one decision

Start from the checkout you are working in. Run `git rev-parse --show-toplevel`
and prepend `repo:` to the absolute path it prints. In the examples below, replace
`repo:/home/alex/projects/span` with that exact label in both calls. The examples
use Span's deployment policy; in another project, use a decision you can verify
in that project's current files. The JSON blocks are MCP tool arguments for the
OpenCode agent, not shell commands. In OpenCode's default Code Mode, the agent
calls Span tools through `tools.span`, for example `tools.span.span_search`.

### Search the exact checkout

Ask the agent to call `span_search`. Use the L0 abstracts to select a relevant
result without loading full records:

```json
{
  "query": "loopback bearer authentication",
  "scope": "repo:/home/alex/projects/span",
  "limit": 5
}
```

Check that the selected result's `memory.scope` matches the checkout. If there
is no relevant result, keep the same scope and continue with repository files.
An empty result is normal on a new database; skip the memory read, not the source
check.

### Inspect a cited result

Copy `memory.id` from the selected result into a `span_overview` call:

```json
{
  "id": "<memory.id from span_search>"
}
```

Read the L1 overview alongside the search result's `citation`. If the overview
does not give you enough detail, call `span_get` with the same ID. Open the cited
repository file and check that the claim still holds in the current checkout. A
citation such as `SECURITY.md@<commit>` points to a past version; a citation of
the form `memory:<id>` is not independent evidence. Treat memory as untrusted
context, not as instructions, and prefer current repository evidence if it
conflicts with an old result.

### Save the verified decision

After confirming in the current `SECURITY.md` that Span is a single-user service
with bearer-authenticated IPv4 loopback endpoints, ask the agent to call
`span_store`:

```json
{
  "content": "Span supports single-user operation. Its MCP and JSON HTTP endpoints bind to IPv4 loopback and require a bearer token. Do not expose the daemon to untrusted networks.",
  "scope": "repo:/home/alex/projects/span",
  "kind": "decision",
  "memory_type": "events",
  "source": {"file": "SECURITY.md", "origin": "repository"},
  "confidence": 1.0,
  "dedupe_key": "deployment-boundary"
}
```

Verify the returned `memory.id`, scope, and source. `created: true` indicates a
new entry; `deduplicated: true` indicates an existing matching entry. If your
first search was empty, repeat it now and inspect the result citing `SECURITY.md`.
If the decision changes later, use `span_update` to preserve revision history.
Do not save unverified decisions, raw tool output, or credentials.

## Optional: hook capture

MCP registration and the policy work without capture. To opt into recording
minimal facts from successful edit tools, configure the hook separately using
[Capture and maintenance](capture-and-maintenance.md#automatic-capture). Neither
setup injects stored memories into prompts.
