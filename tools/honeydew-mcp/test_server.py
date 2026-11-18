import json
import os
import socket
import threading
import unittest
from unittest.mock import patch

import server


class McpTests(unittest.TestCase):
    def test_initialize_and_tools(self):
        initialized = server.handle({"method": "initialize", "params": {"protocolVersion": "2025-03-26"}})
        self.assertEqual(initialized["serverInfo"]["name"], "honeydew")
        self.assertEqual(len(server.handle({"method": "tools/list"})["tools"]), 8)

    def test_get_song_forwards_to_loopback_with_token(self):
        listener = socket.socket()
        listener.bind(("127.0.0.1", 0))
        listener.listen(1)
        port = listener.getsockname()[1]
        received = []

        def serve():
            conn, _ = listener.accept()
            with conn:
                data = b""
                while chunk := conn.recv(4096):
                    data += chunk
                received.append(json.loads(data))
                conn.sendall(b'{"result":{"title":"Test Song"}}')
            listener.close()

        thread = threading.Thread(target=serve)
        thread.start()
        with patch.dict(os.environ, {"HONEYDEW_CONTROL_TOKEN": "a" * 32,
                                  "HONEYDEW_CONTROL_PORT": str(port)}):
            result = server.handle({"method": "tools/call", "params": {
                "name": "get_song", "arguments": {}
            }})
        thread.join(timeout=3)
        self.assertFalse(result["isError"])
        self.assertEqual(json.loads(result["content"][0]["text"])["title"], "Test Song")
        self.assertEqual(received[0], {"token": "a" * 32, "action": "song"})

    def test_instrument_parameter_tools_forward_ids_and_changes(self):
        requests = []

        def fake_honeydew(action, payload=None, channel_id=None):
            requests.append((action, payload, channel_id))
            return {"result": {"ok": True}}

        with patch.object(server, "honeydew", fake_honeydew):
            for name, arguments in [
                ("get_instrument_params", {"channel_id": 7}),
                ("set_instrument_params", {"channel_id": 7, "module_id": 2,
                                           "changes": [{"index": 9, "value": 0.7}]}),
            ]:
                result = server.handle({"method": "tools/call", "params": {
                    "name": name, "arguments": arguments
                }})
                self.assertFalse(result["isError"])
        self.assertEqual(requests, [
            ("instrument_params", None, 7),
            ("set_instrument_params", {"channel_id": 7, "module_id": 2,
                                       "changes": [{"index": 9, "value": 0.7}]}, None),
        ])



    def test_compose_song_uses_composer_and_returns_result(self):
        expected = {"created_tracks": [{"channel_id": 4}]}
        with patch.object(server.composer, "compose", return_value=expected) as compose:
            result = server.handle({"method": "tools/call", "params": {
                "name": "compose_song", "arguments": {
                    "prompt": "warm house", "bars": 4, "seed": 11, "tempo_bpm": 120
                }
            }})
        self.assertFalse(result["isError"])
        self.assertEqual(json.loads(result["content"][0]["text"]), expected)
        compose.assert_called_once_with("warm house", 4, 11, 120, server.honeydew)

    def test_apply_sound_design_forwards_effect_chain(self):
        request = {"channel_id": 12, "effects": [
            {"name": "Reverb", "changes": [{"index": -2, "value": 0.4}]}
        ]}
        with patch.object(server, "honeydew", return_value={"result": {"effect_module_ids": [3]}}) as call:
            result = server.handle({"method": "tools/call", "params": {
                "name": "apply_sound_design", "arguments": request
            }})
        self.assertFalse(result["isError"])
        call.assert_called_once_with("apply_sound_design", request, channel_id=None)

    def test_effect_tools_forward_channel_and_parameters(self):
        requests = []
        def fake_honeydew(action, payload=None, channel_id=None):
            requests.append((action, payload, channel_id))
            return {"result": {"ok": True}}
        with patch.object(server, "honeydew", fake_honeydew):
            for name, arguments in [
                ("list_effects", {"channel_id": 7}),
                ("set_effect_params", {"channel_id": 7, "module_id": 4,
                    "changes": [{"index": 3, "value": 0.65}]}),
            ]:
                result = server.handle({"method": "tools/call", "params": {"name": name, "arguments": arguments}})
                self.assertFalse(result["isError"])
        self.assertEqual(requests, [
            ("effects", None, 7),
            ("set_effect_params", {"channel_id": 7, "module_id": 4,
                "changes": [{"index": 3, "value": 0.65}]}, None),
        ])


if __name__ == "__main__":
    unittest.main()
