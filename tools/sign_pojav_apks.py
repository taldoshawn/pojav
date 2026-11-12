#!/usr/bin/env python3
"""Verify CI provenance and sign APKs locally with the owner's persistent key.

Keys and password files must stay outside Git and outside the output directory.
The original CI artifacts are retained unchanged. This does not publish a release.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import zipfile

from check_android_release import audit

MACHINES = {'arm64-v8a': 183, 'armeabi-v7a': 40, 'x86': 3, 'x86_64': 62}
APP_ID = 'dev.taldoshawn.pojav'


def sha256(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def inspect(source, metadata, commit):
    abi = metadata.get('abi')
    if abi not in MACHINES or metadata.get('commit') != commit or metadata.get('package') != APP_ID:
        raise ValueError('Unexpected CI commit, architecture or application identity')
    if sha256(source) != metadata.get('sha256'):
        raise ValueError('CI APK does not match its recorded SHA-256')
    with zipfile.ZipFile(source) as archive:
        names = archive.namelist()
        native = f'lib/{abi}/libhorizon_android.so'
        if native not in names or any(name.startswith('lib/') and name.endswith('.so')
                                      and not name.startswith(f'lib/{abi}/') for name in names):
            raise ValueError('APK contains an unexpected native architecture')
        with archive.open(native) as stream:
            header = stream.read(20)
        if header[:4] != b'\x7fELF' or header[5] != 1 or int.from_bytes(header[18:20], 'little') != MACHINES[abi]:
            raise ValueError('Native ELF header does not match the advertised architecture')
    return abi


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--artifacts', required=True, type=Path)
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--commit', required=True)
    parser.add_argument('--version', required=True)
    parser.add_argument('--keystore', required=True, type=Path)
    parser.add_argument('--password-file', required=True, type=Path)
    parser.add_argument('--alias', default='pojav-rust')
    parser.add_argument('--build-tools', required=True, type=Path)
    args = parser.parse_args()
    if not re.fullmatch(r'[0-9a-f]{40}', args.commit):
        parser.error('--commit must be the full build commit')
    if not re.fullmatch(r'[0-9]+\.[0-9]+\.[0-9]+(?:-[a-z0-9.]+)?', args.version):
        parser.error('--version must be a numeric release version')
    output = args.output.resolve()
    for private in (args.keystore, args.password_file):
        if not private.is_file() or private.resolve().is_relative_to(output):
            parser.error('Signing files must exist outside the release output')
    packages = {}
    for build in args.artifacts.rglob('BUILD.json'):
        metadata = json.loads(build.read_text())
        sources = list(build.parent.glob('*.apk'))
        if len(sources) != 1:
            raise ValueError('Each CI artifact must contain exactly one APK')
        source = sources[0]
        abi = inspect(source, metadata, args.commit)
        if abi in packages:
            raise ValueError('Duplicate artifact architecture')
        packages[abi] = (source, metadata)
    if set(packages) != set(MACHINES):
        raise ValueError('All four architecture artifacts are required')
    output.mkdir(parents=True, exist_ok=True)
    signer = str(args.build_tools / 'apksigner')
    results = []
    for abi, (source, metadata) in sorted(packages.items()):
        destination = output / f'Pojav-Rust-{args.version}-{abi}.apk'
        if destination.exists():
            raise FileExistsError(destination)
        subprocess.run([signer, 'sign', '--ks', str(args.keystore), '--ks-key-alias', args.alias,
                        '--ks-pass', 'file:' + str(args.password_file),
                        '--key-pass', 'file:' + str(args.password_file),
                        '--out', str(destination), str(source)], check=True)
        verification = subprocess.check_output([signer, 'verify', '--verbose', '--print-certs', str(destination)], text=True)
        (output / f'{abi}-signature.txt').write_text(verification)
        report = audit(destination, args.build_tools / 'aapt2', public=True, app_id=APP_ID)
        (output / f'{abi}-audit.json').write_text(json.dumps(report, indent=2) + '\n')
        if not report['passed']:
            raise ValueError(f'Public APK audit failed for {abi}')
        results.append({**metadata, 'file': destination.name, 'sha256': sha256(destination),
                        'ci_sha256': metadata['sha256'], 'signing': 'owner-persistent-key'})
    (output / 'SHA256SUMS.txt').write_text(''.join(f'{row["sha256"]}  {row["file"]}\n' for row in results))
    (output / 'BUILD.json').write_text(json.dumps({'commit':args.commit, 'version':args.version,
                                                  'apks':results}, indent=2) + '\n')
    print(f'Verified and signed {len(results)} APKs in {output}')


if __name__ == '__main__':
    main()
