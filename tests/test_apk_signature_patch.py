import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('apk_signature_patch', Path(__file__).parents[1] / 'tools/apk_signature_patch.py')
patcher = importlib.util.module_from_spec(spec)
spec.loader.exec_module(patcher)


class SignaturePatchTest(unittest.TestCase):
    def test_round_trip_with_shifted_payload_and_new_signature(self):
        with tempfile.TemporaryDirectory() as directory:
            old = Path(directory) / 'original.apk'
            signed = Path(directory) / 'signed.apk'
            for path, extra in ((old, b''), (signed, b'\xfe\xca\x04\x00test')):
                with zipfile.ZipFile(path, 'w', compression=zipfile.ZIP_DEFLATED) as archive:
                    item = zipfile.ZipInfo('lib/arm64-v8a/libgame.so')
                    item.extra = extra
                    archive.writestr(item, bytes(range(256)) * 100)
                    archive.writestr('META-INF/CERT.SF', b'old' if path == old else b'new signature')
            patch = patcher.create(old, signed)
            self.assertTrue(any('copy' in op for op in patch['operations']))
            self.assertEqual(patcher.apply(old, patch), signed.read_bytes())
            damaged = copy.deepcopy(patch)
            damaged['signed_sha256'] = '0' * 64
            with self.assertRaisesRegex(ValueError, 'hash mismatch'):
                patcher.apply(old, damaged)
            damaged = copy.deepcopy(patch)
            damaged['operations'][0] = {'copy': [-1, 4]}
            with self.assertRaisesRegex(ValueError, 'copy range'):
                patcher.apply(old, damaged)
            old.write_bytes(old.read_bytes() + b'corrupted')
            with self.assertRaisesRegex(ValueError, 'source APK hash'):
                patcher.apply(old, patch)


if __name__ == '__main__':
    unittest.main()
