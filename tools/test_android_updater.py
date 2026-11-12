#!/usr/bin/env python3
"""Run the actual Android updater's pure unit tests on the host, without network calls."""
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def main():
    harness = ROOT / '.local/android/updater-tests'
    (harness / 'src').mkdir(parents=True, exist_ok=True)
    (harness / 'Cargo.toml').write_text('''[package]
name = "pojav-updater-tests"
version = "0.0.0"
edition = "2024"
[workspace]
[dependencies]
serde_json = "1"
log = "0.4"
ureq = { version = "2", default-features = false, features = ["json", "tls"] }
''')
    source = ROOT / 'crates/horizon-client/src/android/updater.rs'
    (harness / 'src/lib.rs').write_text(
        '#![allow(dead_code)]\n#[path = "' + source.as_posix() + '"] mod updater;\n')
    env = dict(os.environ, CARGO_INCREMENTAL='0', CARGO_PROFILE_DEV_DEBUG='0')
    subprocess.run(['cargo', 'test', '--manifest-path', str(harness / 'Cargo.toml')],
                   cwd=ROOT, env=env, check=True)


if __name__ == '__main__':
    main()
