#!/usr/bin/env python3
"""Exercise two independent MCP clients and capture against one Span daemon."""

from contextlib import closing
import http.client
import json
import os
import pathlib
import re
import select
import subprocess
import sys
import tempfile
import time


def main() -> None:
    binary = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "target/debug/span").resolve()
    with tempfile.TemporaryDirectory(prefix="span-smoke-") as directory:
        environment = dict(os.environ, HOME=directory, XDG_DATA_HOME=f"{directory}/data",
                           SPAN_EMBEDDER="hash", SPAN_DB=f"{directory}/memory.db", RUST_LOG="span=info")
        environment.pop("SPAN_HTTP_TOKEN", None)
        for key in ("SPAN_RERANK_URL", "SPAN_RERANK_TOKEN", "SPAN_RERANK_MODEL", "SPAN_VECTOR_INDEX"):
            environment.pop(key, None)
        paths = json.loads(subprocess.check_output([binary, "paths", "--json"], env=environment))
        pid_file = pathlib.Path(paths["daemon_pid"])
        assert pid_file.is_relative_to(directory), pid_file
        invalid_db = pathlib.Path(directory) / "invalid.db"
        invalid = subprocess.run([binary, "serve", "--port", "0"], text=True, capture_output=True,
                                 env=dict(environment, SPAN_HTTP_TOKEN="café", SPAN_DB=str(invalid_db)), timeout=10)
        assert invalid.returncode != 0 and "nonempty ASCII bearer token" in invalid.stderr
        assert "café" not in invalid.stderr and not invalid_db.exists() and not pid_file.exists()
        pid_file.parent.mkdir(parents=True, exist_ok=True)
        pid_file.write_text(f"{os.getpid()}\n")  # An unlocked PID file is stale even if its PID is alive.
        with subprocess.Popen([binary, "serve", "--port", "0"], env=environment, stderr=subprocess.PIPE) as process:
            try:
                deadline = time.monotonic() + 15
                while True:
                    assert select.select([process.stderr], [], [], max(0, deadline - time.monotonic()))[0], "Span startup timed out"
                    line = process.stderr.raw.readline()
                    assert line, "Span quit before listening"
                    token = re.fullmatch(rb"Generated SPAN_HTTP_TOKEN=([0-9a-f]{64})\n", line)
                    if token:
                        environment["SPAN_HTTP_TOKEN"] = token[1].decode("ascii")
                    match = re.search(rb"authenticated loopback HTTP server ready port=(\d+)", line)
                    if match:
                        port = int(match[1])
                        break
                assert "SPAN_HTTP_TOKEN" in environment, "Span did not print a generated token"
                assert int(pid_file.read_text()) == process.pid
                environment["SPAN_HTTP_PORT"] = str(port)
                # A deliberately invalid client model proves the CLI uses the daemon's loaded model.
                cli_env = dict(environment, SPAN_EMBEDDER="invalid", SPAN_DB=f"{directory}/ignored.db")
                def cli(*args, payload=None):
                    return subprocess.check_output([binary, *args], env=cli_env, input=payload, text=True, timeout=10)

                assert "cam-core-feature-hash" in cli("prewarm")
                assert json.loads(cli("doctor", "--json"))["database"] == environment["SPAN_DB"]
                cli("capture", payload=json.dumps({"scope": "global", "kind": "fact", "content": "daemon CLI smoke test"}))
                event = {"scope": "global", "event": "decision", "event_id": "cli-smoke", "content": "daemon CLI capture event", "successful": True}
                cli("capture-event", payload=json.dumps(event))
                assert json.loads(cli("reindex"))["reindexed"] == 2
                assert not pathlib.Path(cli_env["SPAN_DB"]).exists()
                second_db = pathlib.Path(directory) / "second.db"
                duplicate = subprocess.run([binary, "serve", "--port", "0"], text=True, capture_output=True,
                                           env=dict(environment, SPAN_DB=str(second_db), SPAN_EMBEDDER="invalid"), timeout=10)
                assert duplicate.returncode != 0 and "already running" in duplicate.stderr
                assert f"PID {process.pid}" in duplicate.stderr and not second_db.exists()
                assert int(pid_file.read_text()) == process.pid

                def client():
                    return http.client.HTTPConnection("127.0.0.1", port, timeout=10)

                def send(connection, path, payload, *, auth=True, extra=None):
                    headers = {"Content-Type": "application/json", "Accept": "application/json, text/event-stream"}
                    if auth:
                        headers["Authorization"] = "Bearer " + environment["SPAN_HTTP_TOKEN"]
                    headers.update(extra or {})
                    connection.request("POST", path, json.dumps(payload), headers)
                    response = connection.getresponse()
                    raw = response.read()
                    if response.status >= 400:
                        connection.close()
                    return response.status, raw

                def rpc(connection, method, params, request_id=None, version=None):
                    payload = {"jsonrpc": "2.0", "method": method, "params": params}
                    if request_id is not None:
                        payload["id"] = request_id
                    extra = {"MCP-Protocol-Version": version} if version else None
                    status, raw = send(connection, "/mcp", payload, extra=extra)
                    if request_id is None:
                        assert status == 202, (status, raw)
                        return None
                    assert status == 200, (status, raw)
                    reply = json.loads(raw)
                    assert reply["id"] == request_id and "result" in reply, reply
                    return reply["result"]

                def initialize(connection):
                    result = rpc(connection, "initialize", {
                        "protocolVersion": "2025-11-25", "capabilities": {},
                        "clientInfo": {"name": "span-smoke", "version": "1"},
                    }, 1)
                    assert result["serverInfo"]["name"] == "span"
                    version = result["protocolVersion"]
                    rpc(connection, "notifications/initialized", {}, version=version)
                    return version

                def tool(connection, version, name, arguments):
                    result = rpc(connection, "tools/call", {"name": name, "arguments": arguments}, 3, version)
                    assert not result.get("isError", False), result
                    return result.get("structuredContent") or json.loads(result["content"][0]["text"])

                with closing(client()) as first, closing(client()) as second:
                    assert send(first, "/mcp", {}, auth=False)[0] == 401
                    assert send(first, "/mcp", {}, extra={"Authorization": "Bearer wrong"})[0] == 401
                    assert send(first, "/mcp", {}, extra={"Origin": "http://localhost"})[0] == 403
                    first.request("POST", "/mcp", headers={
                        "Authorization": "Bearer " + environment["SPAN_HTTP_TOKEN"],
                        "Content-Length": "1048577",
                    })
                    oversized = first.getresponse()
                    assert oversized.status == 413
                    oversized.read()
                    first.close()
                    first_version, second_version = initialize(first), initialize(second)
                    tools = rpc(first, "tools/list", {}, 2, first_version)["tools"]
                    assert all("outputSchema" not in tool or tool["outputSchema"].get("type") == "object"
                               for tool in tools), "MCP output schemas must describe objects"
                    names = {tool["name"] for tool in tools}
                    assert names == set("""span_search span_search_diagnostics span_overview span_get span_batch_get
                        span_history span_timeline span_context span_store span_update span_forget span_link span_touch
                        span_consolidate span_evidence span_pages span_page_status span_refresh_page span_reflect
                        span_compact span_contradictions span_maintenance span_record_revision span_revisions
                        span_handoff_create span_handoff_claim span_handoff_list span_capture_event""".split()), names
                    scope = f"repo:{directory}"
                    saved = tool(first, first_version, "span_store", {
                        "content": "Authentication uses rotating refresh credentials.", "scope": scope, "kind": "decision",
                    })
                    loaded = tool(second, second_version, "span_get", {"id": saved["memory"]["id"]})
                    assert loaded["memory"]["content"] == saved["memory"]["content"]
                    batch = tool(second, second_version, "span_batch_get", {"scope": scope, "ids": [saved["memory"]["id"]]})
                    assert batch["data"] == [loaded], batch
                    hook_env = dict(environment, SPAN_CAPTURE="1", SPAN_HTTP_PORT=str(port), SPAN_CAPTURE_QUEUE=f"{directory}/queue")
                    subprocess.run([sys.executable, pathlib.Path(__file__).with_name("capture_hook.py")],
                                   input=json.dumps({"cwd": directory, "tool_name": "Edit", "session_id": "smoke", "call_id": "1"}),
                                   text=True, check=True, env=hook_env, timeout=10)
                    hits = tool(second, second_version, "span_search", {"scope": scope, "kinds": ["task_result"], "query": "successful Edit"})
                    assert "A successful Edit operation" in json.dumps(hits), hits
                    assert not list(pathlib.Path(hook_env["SPAN_CAPTURE_QUEUE"]).glob("*.json"))
                print("MCP smoke test passed: two clients, 28 tools, shared writes, authentication, and HTTP capture")
            finally:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                    raise


if __name__ == "__main__":
    main()
