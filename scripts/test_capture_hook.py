import concurrent.futures
import json
import pathlib
import tempfile
import time
import unittest
from unittest.mock import patch

from capture_hook import DaemonUnavailable, drain, drain_with_alarm, enqueue, normalize


class CaptureTests(unittest.TestCase):
    def test_does_not_capture_arguments_or_output(self) -> None:
        event = {"tool_name": "Edit", "session_id": "session", "tool_use_id": "call", "tool_input": {"secret": "sensitive"}, "tool_response": "confidential"}
        captured = normalize(event)
        self.assertIsNotNone(captured)
        self.assertNotIn("sensitive", str(captured))
        self.assertNotIn("confidential", str(captured))
        self.assertEqual(captured, normalize(event))

    def test_skips_failed_or_unapproved_tools(self) -> None:
        self.assertIsNone(normalize({"tool_name": "Bash"}))
        self.assertIsNone(normalize({"tool_name": "Edit", "successful": False}))

    def test_malformed_file_does_not_block_later_events(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            queue = pathlib.Path(directory)
            malformed = queue / "000.json"
            malformed.write_bytes(b"{")
            event = normalize({"tool_name": "Edit", "session_id": "s", "tool_use_id": "call"})
            enqueue(queue, event)
            with patch("capture_hook.ingest") as run:
                self.assertFalse(drain(queue))
                run.assert_called_once()
                self.assertEqual(json.loads(run.call_args.args[0]), event)
            self.assertEqual(list(queue.glob("*.json")), [])
            [quarantined] = (queue / "quarantine").glob("*/000.json")
            self.assertEqual(quarantined.read_bytes(), b"{")

    def test_drain_continues_after_ingestion_failure(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            queue = pathlib.Path(directory)
            events = [normalize({"tool_name": "Edit", "session_id": "s", "tool_use_id": str(n)})
                      for n in range(2)]
            for event in events:
                enqueue(queue, event)

            def ingest(raw: bytes, timeout: float) -> None:
                if json.loads(raw) == events[0]:
                    raise OSError("Span unavailable")

            with patch("capture_hook.ingest", side_effect=ingest) as run:
                self.assertFalse(drain(queue))
                self.assertEqual(run.call_count, 2)
            [pending] = queue.glob("*.json")
            self.assertEqual(json.loads(pending.read_bytes()), events[0])
            with patch("capture_hook.ingest") as run:
                self.assertTrue(drain(queue))
                run.assert_called_once()
            self.assertFalse(pending.exists())

    def test_concurrent_drainers_only_ingest_once(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            queue = pathlib.Path(directory)
            event = normalize({"tool_name": "Edit", "session_id": "s", "tool_use_id": "call"})
            enqueue(queue, event)
            enqueue(queue, event)
            with patch("capture_hook.ingest", side_effect=lambda raw, timeout: time.sleep(0.05)) as run:
                with concurrent.futures.ThreadPoolExecutor(max_workers=2) as pool:
                    self.assertEqual(list(pool.map(drain, [queue, queue])), [True, True])
                run.assert_called_once()
            self.assertEqual(list(queue.glob("*.json")), [])

    def test_daemon_failure_stops_drain_and_keeps_all_events(self) -> None:
        for error in ("daemon returned HTTP 401", "could not reach the Span daemon"):
            with self.subTest(error=error), tempfile.TemporaryDirectory() as directory:
                queue = pathlib.Path(directory)
                for call in ("first", "second"):
                    enqueue(queue, normalize({"tool_name": "Edit", "session_id": "s", "call_id": call}))
                with patch("capture_hook.ingest", side_effect=DaemonUnavailable(error)) as run:
                    self.assertFalse(drain(queue))
                    run.assert_called_once()
                self.assertEqual(len(list(queue.glob("*.json"))), 2)

    def test_alarm_interrupts_slow_ingestion_and_keeps_event(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            queue = pathlib.Path(directory)
            enqueue(queue, normalize({"tool_name": "Edit", "session_id": "s", "call_id": "slow"}))
            with patch("capture_hook.DRAIN_SECONDS", 0.05), patch("capture_hook.ingest", side_effect=lambda raw, timeout: time.sleep(1)) as run:
                self.assertFalse(drain_with_alarm(queue))
                self.assertLessEqual(run.call_args.args[1], 0.05)
            self.assertEqual(len(list(queue.glob("*.json"))), 1)


if __name__ == "__main__":
    unittest.main()
