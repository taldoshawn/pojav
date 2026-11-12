"""Reject mixed or modified artifacts before any signing key is used."""
import importlib.util
import hashlib
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'tools'))
import sign_pojav_apks as signing


class ArtifactProvenanceTests(unittest.TestCase):
    def test_architecture_and_provenance_are_independently_checked(self):
        commit = 'a' * 40
        with tempfile.TemporaryDirectory() as temporary:
            apk = Path(temporary) / 'test.apk'
            header = bytearray(20)
            header[:6] = b'\x7fELF\x02\x01'
            header[18:20] = (183).to_bytes(2, 'little')
            with zipfile.ZipFile(apk, 'w') as archive:
                archive.writestr('lib/arm64-v8a/libhorizon_android.so', header)
            metadata = {'commit':commit, 'abi':'arm64-v8a', 'package':signing.APP_ID,
                        'sha256':hashlib.sha256(apk.read_bytes()).hexdigest()}
            self.assertEqual(signing.inspect(apk, metadata, commit), 'arm64-v8a')
            for changed in ({'commit':'b'*40}, {'abi':'x86'}, {'sha256':'0'*64}, {'package':'other.app'}):
                with self.assertRaises(ValueError):
                    signing.inspect(apk, {**metadata, **changed}, commit)
            header[18:20] = (62).to_bytes(2, 'little')
            with zipfile.ZipFile(apk, 'w') as archive:
                archive.writestr('lib/arm64-v8a/libhorizon_android.so', header)
            metadata['sha256'] = hashlib.sha256(apk.read_bytes()).hexdigest()
            with self.assertRaises(ValueError):
                signing.inspect(apk, metadata, commit)


if __name__ == '__main__':
    unittest.main()
