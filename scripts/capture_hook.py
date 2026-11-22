#!/usr/bin/env python3
"""Capture successful harness tool events without persisting raw arguments or output.

Opt in with SPAN_CAPTURE=1. Configure this command as a successful post-tool
hook in the harness. Failed ingestion is reported and retained locally for retry.
"""

import fcntl
import hashlib
import json
import os
import pathlib
import signal
import sys
import tempfile
import time
import urllib.error
import urllib.request


MAX_PAYLOAD = 1_048_576
DRAIN_SECONDS = 5.0


class DaemonUnavailable(Exception):
    """Draining further events will not help until the daemon is available."""


def normalize(event: dict) -> dict | None:
    """Store a minimal activity fact, never transcript text or tool payloads."""
    if event.get("error") or event.get("successful") is False:
        return None
    tool = event.get("tool_name", event.get("tool"))
    if tool not in {"Edit", "Write", "apply_patch", "edit", "write", "patch"}:
        return None
    directory = pathlib.Path(event.get("cwd", os.getcwd())).resolve()
    session = str(event.get("session_id", ""))
    call = str(event.get("tool_use_id", event.get("call_id", "")))
    if not session or not call:
        raise ValueError("Capture requires session_id and tool_use_id/call_id")
    identity = hashlib.sha256(f"{directory}:{session}:{call}".encode()).hexdigest()
    return {
        "event_id": identity,
        "scope": f"repo:{directory}",
        "event": "task_complete",
        "successful": True,
        "content": f"A successful {tool} operation completed in this repository. Verify repository files before relying on this activity record.",
    }


def enqueue(queue: pathlib.Path, event: dict) -> None:
    pending = queue / f"{event['event_id']}.json"
    with tempfile.NamedTemporaryFile(mode="w", dir=queue, prefix=".pending-", delete=False) as stream:
        staged = pathlib.Path(stream.name)
        try:
            json.dump(event, stream)
            stream.flush()
            os.fsync(stream.fileno())
            try:
                os.link(staged, pending)
            except FileExistsError:
                pass
        finally:
            staged.unlink()


def read_event(path: pathlib.Path) -> bytes:
    with path.open("rb") as stream:
        raw = stream.read(MAX_PAYLOAD + 1)
    if len(raw) > MAX_PAYLOAD:
        raise ValueError("queued payload exceeds 1 MiB")
    fields = json.loads(raw)
    if (
        not isinstance(fields, dict)
        or not all(isinstance(fields.get(key), str) for key in ("event_id", "scope", "event", "content"))
        or not isinstance(fields.get("successful"), bool)
    ):
        raise ValueError("invalid queued event fields")
    return raw


def drain(queue: pathlib.Path) -> bool:
    deadline = time.monotonic() + DRAIN_SECONDS
    descriptor = os.open(queue / ".drain.lock", os.O_CREAT | os.O_RDWR, 0o600)
    with os.fdopen(descriptor, "rb") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return True
        return drain_locked(queue, deadline)


def ingest(raw: bytes, timeout: float) -> None:
    token = os.environ.get("SPAN_HTTP_TOKEN", "")
    if not token.strip():
        raise DaemonUnavailable("SPAN_HTTP_TOKEN must be set")
    try:
        port = int(os.environ.get("SPAN_HTTP_PORT", "8787"))
    except ValueError:
        raise DaemonUnavailable("SPAN_HTTP_PORT must be a valid listening port") from None
    if not 1 <= port <= 65535:
        raise DaemonUnavailable("SPAN_HTTP_PORT must be a valid listening port")

    class NoRedirect(urllib.request.HTTPRedirectHandler):
        def redirect_request(self, req, fp, code, msg, headers, newurl):
            return None

    request = urllib.request.Request(
        f"http://127.0.0.1:{port}/capture-event", data=raw,
        headers={"Authorization": f"Bearer {token}", "Content-Type": "application/json"},
    )
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    try:
        with opener.open(request, timeout=timeout) as response:
            if response.status != 200:
                raise DaemonUnavailable(f"unexpected HTTP status {response.status}")
    except urllib.error.HTTPError as error:
        error.close()
        if error.code in (400, 413, 422):
            raise ValueError(f"daemon rejected event (HTTP {error.code})") from None
        raise DaemonUnavailable(f"daemon returned HTTP {error.code}") from None
    except (OSError, ValueError) as error:
        raise DaemonUnavailable("could not reach the Span daemon; check its port and token") from error


def drain_locked(queue: pathlib.Path, deadline: float) -> bool:
    failed = False
    # Prioritize unattempted events over older files that have already failed.
    pending = sorted(queue.glob("*.json"), key=lambda path: (path.stat().st_mtime_ns, path.name))
    for path in pending[:100]:
        if time.monotonic() >= deadline:
            return not failed
        try:
            raw = read_event(path)
        except ValueError:
            failed = True
            quarantine = queue / "quarantine"
            quarantine.mkdir(mode=0o700, exist_ok=True)
            destination = pathlib.Path(tempfile.mkdtemp(prefix="invalid-", dir=quarantine)) / path.name
            path.rename(destination)
            print(f"Span capture: quarantined malformed event at {destination}", file=sys.stderr)
            continue
        except OSError:
            failed = True
            print(f"Span capture: could not read {path.name}; retained for retry", file=sys.stderr)
            continue
        try:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return not failed
            ingest(raw, remaining)
        except DaemonUnavailable as error:
            print(f"Span capture: {error}; pending events retained for retry", file=sys.stderr)
            return False
        except (OSError, ValueError):
            failed = True
            os.utime(path, None)
            print(f"Span capture: ingestion failed for {path.name}; retained for retry", file=sys.stderr)
            continue
        path.unlink()
    return not failed


def drain_with_alarm(queue: pathlib.Path) -> bool:
    def expired(_signum: int, _frame: object) -> None:
        raise DaemonUnavailable("drain time budget expired")

    previous = signal.signal(signal.SIGALRM, expired)
    try:
        signal.setitimer(signal.ITIMER_REAL, DRAIN_SECONDS)
        return drain(queue)
    except DaemonUnavailable as error:
        print(f"Span capture: {error}; pending events retained for retry", file=sys.stderr)
        return False
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        signal.signal(signal.SIGALRM, previous)


def main() -> None:
    if os.environ.get("SPAN_CAPTURE") != "1":
        return
    raw = sys.stdin.buffer.read(MAX_PAYLOAD + 1)
    if len(raw) > MAX_PAYLOAD:
        raise ValueError("Hook payload exceeds 1 MiB")
    event = normalize(json.loads(raw))
    if event is None:
        return
    queue = pathlib.Path(os.environ.get("SPAN_CAPTURE_QUEUE", "~/.local/state/span/capture")).expanduser()
    queue.mkdir(mode=0o700, parents=True, exist_ok=True)
    os.chmod(queue, 0o700)
    enqueue(queue, event)
    if not drain_with_alarm(queue):
        sys.exit(1)


if __name__ == "__main__":
    main()
