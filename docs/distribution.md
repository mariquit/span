# Distribution and harness installation

## Release artifacts

Version-tag pushes publish standalone binaries and SHA-256 checksum files for:

| Target | Runner |
| --- | --- |
| `x86_64-unknown-linux-gnu` | Ubuntu 24.04 |
| `aarch64-unknown-linux-gnu` | Ubuntu 24.04 ARM64 |
| `aarch64-apple-darwin` | macOS Apple Silicon |

Intel macOS (`x86_64-apple-darwin`) is no longer a supported release target because
ONNX Runtime has phased out prebuilt support used by Span's current dependency
stack. The Intel build fails because no compatible prebuilt runtime is available;
macOS releases now target Apple Silicon only. Restoring Intel support would require
a separately maintained runtime build or a tested alternative dependency stack.

Assets are named `span-<target>` and `span-<target>.sha256`. `LICENSE`, `NOTICE`, and
`THIRD-PARTY-NOTICES.md` are attached separately and should accompany downloaded binaries.
There are no generated installers, archives, or distribution manifests.

The embedding model is not embedded in the executable. `span serve` downloads
its pinned revision into the local cache.

## Install from a GitHub Release

Follow the platform-specific download and checksum commands in
[README.md](../README.md#install-a-release). Choose the correct target, verify the
checksum, and place the executable in a directory on your PATH. On Unix, make it
executable. Then start the daemon as described below and run `span doctor` with
the daemon's token to verify its loaded model and database.

Checksums detect accidental corruption; without a separately trusted signature,
they do not independently authenticate the publisher. macOS notarization is not
configured.

## Register coding harnesses

Start one Span daemon with `span serve --port 8787`. If you did not supply a token,
copy the generated token from startup stderr into `SPAN_HTTP_TOKEN` in the harness
environment. For a stable token across restarts, supply `SPAN_HTTP_TOKEN` to the
daemon and harnesses. Then register the shared MCP URL:

```sh
span setup opencode
span setup claude-code
span setup codex
span setup chatgpt-desktop
span setup all
```

Preview without modifying configuration:

```sh
span setup all --dry-run
```

The command invokes each harness's own configuration CLI for
`http://127.0.0.1:8787/mcp`, with an environment reference for the token.
Run `span setup <harness> --port <port>` for a different daemon port. `all` skips
harness CLIs that are not installed.

If upgrading from CAM, check the harness's MCP list for old `cam` or `memory`
entries and project-level overrides. Replace the old entry in its original
scope with the HTTP registration. If the harness refuses to overwrite an
existing entry, remove just that registration before running setup. This changes
harness configuration, not the Span database. Verify that the harness lists Span
as connected and exposes its tools.

### Manual equivalents

```sh
opencode mcp add span --global --url http://127.0.0.1:8787/mcp \
  --header 'Authorization=Bearer {env:SPAN_HTTP_TOKEN}'
claude mcp add-json --scope user span \
  '{"type":"http","url":"http://127.0.0.1:8787/mcp","headers":{"Authorization":"Bearer ${SPAN_HTTP_TOKEN}"}}'
codex mcp add span --url http://127.0.0.1:8787/mcp --bearer-token-env-var SPAN_HTTP_TOKEN
```

ChatGPT Desktop uses the local Codex-host configuration where supported.
ChatGPT web cannot connect directly to this loopback endpoint. Do not expose
the Span daemon through a public listener.

## Persistent files

The binary is separate from persistent state. Upgrades do not remove either
the database or model cache. `span paths` prints the exact locations.

| Platform | Database base | Cache base |
|---|---|---|
| Linux | `$XDG_DATA_HOME` or `~/.local/share` | `$XDG_CACHE_HOME` or `~/.cache` |
| macOS | `~/Library/Application Support` | `~/Library/Caches` |

`SPAN_DB` and `SPAN_MODEL_CACHE` override the discovered paths.

Create a consistent database backup while other clients may be connected:

```sh
span backup ./memory-backup.db
```

Copy that backup to the database path reported by `span paths` on the
destination machine. Do not live-sync the SQLite files with a file-sync tool.

## Cutting a release

1. Update the workspace version and `CHANGELOG.md`.
2. Run the release checks:

   ```sh
   bash scripts/verify.sh full
   ```

3. Commit the release, tag it, and push:

   ```sh
   git tag -s v0.1.0 -m "span v0.1.0"
   git push origin main v0.1.0
   ```

The `.github/workflows/release.yml` workflow uses one native build matrix and
uploads each binary and checksum directly to the tagged GitHub Release. A small
separate job uploads the license and notices. The release can be partially
populated if a target fails; inspect all workflow jobs and expected assets before
announcing it. Tags containing a hyphen are marked as prereleases.

## Release security

- Release builds use `Cargo.lock`.
- The embedding model repository and commit are pinned in source.
- `LICENSE`, `NOTICE`, and `THIRD-PARTY-NOTICES.md` are attached to the release.
- Each binary has a SHA-256 checksum file; artifact attestations are not generated.
- macOS code signing and notarization remain future work.
