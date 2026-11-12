#!/usr/bin/env python3
"""Read-only Java server status diagnostics. Never logs in or registers accounts.

A successful status response establishes reachability, not gameplay compatibility.
"""
import io
import hashlib
import os
import secrets
import zlib
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


def probe_login_start(host, protocol, use_srv_host, name):
    """Compare initial routing only; stop before authentication/configuration."""
    target, port = endpoint(host)
    handshake_host = target if use_srv_host else host
    result = {'host': host, 'endpoint': target, 'handshake_host': handshake_host,
              'requested_protocol': protocol, 'phase': 'login_start_only',
              'authenticated': False, 'gameplay_verified': False}
    try:
        with socket.create_connection((target, port), timeout=8) as sock:
            sock.settimeout(8)
            address = handshake_host.encode()
            hello = b'\x00' + varint(protocol) + varint(len(address)) + address + struct.pack('>H', port) + b'\x02'
            login = b'\x00' + varint(len(name)) + name.encode()
            if protocol == 777:
                identity = bytearray(hashlib.md5(('OfflinePlayer:' + name).encode()).digest())
                identity[6] = (identity[6] & 0x0f) | 0x30
                identity[8] = (identity[8] & 0x3f) | 0x80
                login += identity
            sock.sendall(varint(len(hello)) + hello + varint(len(login)) + login)
            compressed = False
            packets = []
            with sock.makefile('rb') as stream:
                for _ in range(4):
                    size = read_varint(stream.read)
                    if not 1 <= size <= 65536:
                        raise ValueError('Unexpected login frame size')
                    frame = stream.read(size)
                    if len(frame) != size:
                        raise EOFError('Truncated login frame')
                    if compressed:
                        body = io.BytesIO(frame)
                        unpacked_size = read_varint(body.read)
                        frame = body.read()
                        if unpacked_size:
                            if unpacked_size > 65536:
                                raise ValueError('Unexpected decompressed login size')
                            decoder = zlib.decompressobj()
                            frame = decoder.decompress(frame, 65537)
                            if len(frame) != unpacked_size or not decoder.eof:
                                raise ValueError('Invalid compressed login frame')
                    payload = io.BytesIO(frame)
                    packet_id = read_varint(payload.read)
                    packets.append(packet_id)
                    if packet_id == 3:
                        compressed = True
                        continue
                    result['server_packet_ids'] = packets
                    result['response'] = {0:'disconnect', 1:'authentication_requested', 2:'login_success',
                                          4:'plugin_query', 5:'cookie_query'}.get(packet_id, 'other_login_packet')
                    if packet_id == 0:
                        length = read_varint(payload.read)
                        result['disconnect_reason'] = payload.read(min(length, 8192)).decode('utf-8', errors='replace')
                    break
    except (OSError, EOFError, ValueError, zlib.error) as error:
        result.update(error=str(error), error_type=type(error).__name__)
    return result


if __name__ == '__main__':
    results = [probe(host, protocol) for host in ('jogar.mush.com.br', 'mc.hypixel.net') for protocol in (47, 777)]
    if os.environ.get('LOGIN_PROBE') == 'true':
        name = 'PR' + secrets.token_hex(5)
        for protocol in (47, 777):
            for srv in (False, True):
                results.append(probe_login_start('jogar.mush.com.br', protocol, srv, name))
                time.sleep(2)
    Path('server-status.json').write_text(json.dumps(results, ensure_ascii=False, indent=2) + '\n')
    print(json.dumps(results, ensure_ascii=False, indent=2))
