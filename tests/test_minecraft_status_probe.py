import importlib.util
import io
from pathlib import Path
import socket
import threading
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('probe', Path(__file__).parents[1] / 'tools/probe_minecraft_status.py')
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)


class ServerStatusTest(unittest.TestCase):
    def test_status_only_handshake_with_loopback_server(self):
        errors = []
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            listener.listen(1)
            port = listener.getsockname()[1]

            def serve():
                try:
                    conn, _ = listener.accept()
                    with conn:
                        conn.settimeout(2)
                        with conn.makefile('rb') as stream:
                            size = probe.read_varint(stream.read)
                            self.assertEqual(stream.read(size), b'\x00\x2f\x0cexample.test' + port.to_bytes(2, 'big') + b'\x01')
                            self.assertEqual(stream.read(2), b'\x01\x00')
                            data = b'{"version":{"name":"1.8.9","protocol":47},"players":{"online":2}}'
                            conn.sendall(bytes([len(data) + 2, 0, len(data)]) + data)
                            self.assertEqual(stream.read(1), b'')  # No login, chat or registration.
                except Exception as error:
                    errors.append(error)

            worker = threading.Thread(target=serve)
            worker.start()
            with patch.object(probe, 'endpoint', return_value=('127.0.0.1', port)):
                result = probe.probe('example.test', 47)
            worker.join(timeout=5)
            self.assertFalse(worker.is_alive())
            self.assertFalse(errors, errors)
            self.assertTrue(result['reachable'])
            self.assertEqual(result['version']['protocol'], 47)
            self.assertFalse(result['login_attempted'])
            self.assertFalse(result['gameplay_verified'])

    def test_truncated_and_oversized_lengths_are_rejected(self):
        for data in [b'', b'\x80', b'\x80\x80\x80\x80\x80', b'\xff\xff\xff\xff\x7f']:
            with self.assertRaises((ValueError, EOFError)):
                probe.read_varint(io.BytesIO(data).read)


if __name__ == '__main__':
    unittest.main()
