#!/usr/bin/env bash
# Enable repository hooks without silently replacing another hook configuration.
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root"
current="$(git config --get core.hooksPath || true)"
if [[ -n "$current" && "$current" != .githooks ]]; then
  # Tokensave delegates to executable hooks under the repository Git directory.
  # Recognize its exact forwarding script rather than replacing global hooks.
  for hook in pre-commit pre-push; do
    expected="$(printf '#!/bin/sh\n# tokensave: chain-repo-hook\nrepo_hook="$(git rev-parse --git-dir 2>/dev/null)/hooks/%s"\nif [ -x "$repo_hook" ] && [ "$repo_hook" != "$0" ]; then\n\t"$repo_hook" "$@"\nfi' "$hook")"
    if [[ ! -f "$current/$hook" || "$(cat "$current/$hook")" != "$expected" ]]; then
      echo "Existing hooks do not match the supported Tokensave forwarding scripts. No configuration changed." >&2
      exit 1
    fi
  done
  hooks="$(git rev-parse --git-dir)/hooks"
  mkdir -p "$hooks"
  for hook in pre-commit pre-push; do
    if [[ -e "$hooks/$hook" || -L "$hooks/$hook" ]]; then
      if ! cmp -s ".githooks/$hook" "$hooks/$hook"; then
        echo "Existing repository $hook differs from Span's hook. No hooks replaced." >&2
        exit 1
      fi
    fi
  done
  for hook in pre-commit pre-push; do
    if [[ ! -e "$hooks/$hook" ]]; then
      cp ".githooks/$hook" "$hooks/$hook"
    fi
    chmod +x "$hooks/$hook"
  done
  echo "Span hooks installed through Tokensave's repository-hook forwarding. Global configuration preserved."
  exit 0
fi
for hook in pre-commit pre-push; do
  existing="$(git rev-parse --git-path "hooks/$hook")"
  if [[ -z "$current" && -f "$existing" ]]; then
    echo "Existing $hook hook found. Integrate the Span checks manually; no configuration changed." >&2
    exit 1
  fi
done
chmod +x .githooks/pre-commit .githooks/pre-push
git config --local core.hooksPath .githooks
echo "Span hooks enabled: fast checks before commits, full verification before pushes."
