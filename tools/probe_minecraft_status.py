#!/usr/bin/env python3
"""Read-only Java server status diagnostics. Never logs in or registers accounts.

A successful status response establishes reachability, not gameplay compatibility.
"""
import io
import json
from pathlib import Path
import socket
import struct
import subprocess
import time


def varint(value):
    out = bytearray()
    while True:
        part = value & 0x7f
        value >>= 7
        out.append(part | (0x80 if value else 0))
        if not value:
            return bytes(out)


def read_varint(read):
    result = 0
    for index in range(5):
        byte = read(1)
        if not byte:
            raise EOFError('Connection closed before VarInt completed')
        result |= (byte[0] & 0x7f) << (7 * index)
        if not byte[0] & 0x80:
            if result > 0x7fffffff:
                raise ValueError('Negative or oversized length')
            return result
    raise ValueError('VarInt exceeds five bytes')


def endpoint(host):
    try:
        records = subprocess.check_output(['dig', '+short', 'SRV', '_minecraft._tcp.' + host],
                                          text=True, timeout=10).splitlines()
        entries = [line.split() for line in records if len(line.split()) == 4]
        if entries:
            record = min(entries, key=lambda row: int(row[0]))
            return record[3].rstrip('.'), int(record[2])
    except (OSError, ValueError, subprocess.SubprocessError):
        pass
    return host, 25565


def probe(host, protocol):
    target, port = endpoint(host)
    result = {'host': host, 'endpoint': target, 'port': port, 'requested_protocol': protocol,
              'login_attempted': False, 'gameplay_verified': False}
    start = time.monotonic()
    try:
        with socket.create_connection((target, port), timeout=8) as sock:
            sock.settimeout(8)
            address = host.encode('utf-8')
            hello = b'\x00' + varint(protocol) + varint(len(address)) + address + struct.pack('>H', port) + b'\x01'
            sock.sendall(varint(len(hello)) + hello + b'\x01\x00')
            with sock.makefile('rb') as stream:
                size = read_varint(stream.read)
                if not 1 <= size <= 2 * 1024 * 1024:
                    raise ValueError('Invalid status frame size')
                frame = stream.read(size)
                if len(frame) != size:
                    raise EOFError('Truncated status frame')
            payload = io.BytesIO(frame)
            if read_varint(payload.read) != 0:
                raise ValueError('Expected a status response')
            size = read_varint(payload.read)
            raw = payload.read(size)
            if len(raw) != size or payload.read(1):
                raise ValueError('Invalid status JSON length')
            status = json.loads(raw)
            result.update(reachable=True, version=status.get('version'), description=status.get('description'),
                          online=status.get('players', {}).get('online'))
    except (OSError, EOFError, ValueError) as error:
        result.update(reachable=False, error=str(error), error_type=type(error).__name__)
    result['elapsed_ms'] = round((time.monotonic() - start) * 1000)
    return result


if __name__ == '__main__':
    results = [probe(host, protocol) for host in ('jogar.mush.com.br', 'mc.hypixel.net') for protocol in (47, 777)]
    Path('server-status.json').write_text(json.dumps(results, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(results, ensure_ascii=False, indent=2))
