#!/usr/bin/env python3
"""Transport offline APK signatures without uploading signing secrets.

Copy identical compressed ZIP members from the verified CI artifact; store only
changed ZIP headers, signing blocks and metadata as public base64 literals.
Both inputs and the reconstructed APK are pinned by SHA-256. No key is included.
"""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import struct
import zipfile


def digest(data):
    return hashlib.sha256(data).hexdigest()


def payload(data, entry):
    start = entry.header_offset
    if data[start:start + 4] != b'PK\x03\x04':
        raise ValueError('Invalid ZIP local header')
    name_len, extra_len = struct.unpack_from('<HH', data, start + 26)
    begin = start + 30 + name_len + extra_len
    return begin, begin + entry.compress_size


def create(source, signed):
    before, after = source.read_bytes(), signed.read_bytes()
    operations = []
    cursor = 0
    with zipfile.ZipFile(source) as old, zipfile.ZipFile(signed) as new:
        entries = {entry.filename: entry for entry in old.infolist()}
        for entry in sorted(new.infolist(), key=lambda e: e.header_offset):
            original = entries.get(entry.filename)
            if original is None or not entry.compress_size:
                continue
            start, end = payload(after, entry)
            old_start, old_end = payload(before, original)
            if after[start:end] != before[old_start:old_end]:
                continue
            if start < cursor:
                raise ValueError('Overlapping ZIP payloads')
            if start > cursor:
                operations.append({'literal': base64.b64encode(after[cursor:start]).decode('ascii')})
            operations.append({'copy': [old_start, old_end - old_start]})
            cursor = end
    if cursor < len(after):
        operations.append({'literal': base64.b64encode(after[cursor:]).decode('ascii')})
    return {'format': 1, 'source_sha256': digest(before), 'signed_sha256': digest(after),
            'signed_bytes': len(after), 'operations': operations}


def apply(source, patch):
    before = source.read_bytes()
    if patch.get('format') != 1 or digest(before) != patch['source_sha256']:
        raise ValueError('Unexpected patch format or source APK hash')
    limit = patch['signed_bytes']
    if not isinstance(limit, int) or not 0 < limit <= 256 * 1024 * 1024:
        raise ValueError('Invalid output size')
    output = bytearray()
    for operation in patch['operations']:
        if set(operation) == {'copy'}:
            start, count = operation['copy']
            if not all(isinstance(n, int) for n in (start, count)) or min(start, count) < 0 or start + count > len(before):
                raise ValueError('Invalid copy range')
            part = before[start:start + count]
        elif set(operation) == {'literal'}:
            part = base64.b64decode(operation['literal'], validate=True)
        else:
            raise ValueError('Unknown operation')
        if len(output) + len(part) > limit:
            raise ValueError('Patch exceeds expected output size')
        output.extend(part)
    if len(output) != limit or digest(output) != patch['signed_sha256']:
        raise ValueError('Reconstructed APK hash mismatch')
    return output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    make = sub.add_parser('create')
    make.add_argument('source', type=Path)
    make.add_argument('signed', type=Path)
    make.add_argument('patch', type=Path)
    restore = sub.add_parser('apply')
    restore.add_argument('source', type=Path)
    restore.add_argument('patch', type=Path)
    restore.add_argument('output', type=Path)
    args = parser.parse_args()
    if args.command == 'create':
        data = create(args.source, args.signed)
        with args.patch.open('x') as file:
            json.dump(data, file, separators=(',', ':'))
            file.write('\n')
    else:
        data = apply(args.source, json.loads(args.patch.read_text()))
        with args.output.open('xb') as file:
            file.write(data)


if __name__ == '__main__':
    main()
