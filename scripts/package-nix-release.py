#!/usr/bin/env python3
"""Build a signed, flat Nix cache suitable for GitHub release assets.

The signing key stays outside the checkout and is never copied into artifacts.
Every runtime dependency is included; clients download only missing store paths.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
from urllib.parse import urlencode

ROOT = Path(__file__).resolve().parents[1]


def prepare(output, installer, version, secret_key):
    if not re.fullmatch(r'v[0-9]+\.[0-9]+\.[0-9]+(?:[.-][A-Za-z0-9.-]+)?', version):
        raise ValueError('Use a version such as v0.1.0')
    if output.exists() and any(output.iterdir()):
        raise ValueError('Output directory must be empty; published versions are immutable')
    if secret_key.stat().st_mode & 0o077:
        raise ValueError('Signing key must not be readable by other users')
    public = (ROOT / 'packaging/nix/cache-public-key').read_text().strip()
    # Check key identity without printing the secret.
    derived = subprocess.check_output(['nix', '--extra-experimental-features', 'nix-command',
                                      'key', 'convert-secret-to-public'], input=secret_key.read_bytes()).decode().strip()
    if derived != public:
        raise ValueError('Signing key does not match the public key pinned in the installer')
    output.mkdir(parents=True, exist_ok=True)
    query = urlencode({'compression': 'xz', 'compression-level': '3', 'secret-key': str(secret_key.resolve())})
    subprocess.run(['nix', '--extra-experimental-features', 'nix-command flakes', 'copy',
                    '--to', output.resolve().as_uri() + '?' + query, str(installer)], check=True)
    # GitHub release assets have flat filenames. NAR URLs are not part of the
    # signed fingerprint (store path, NAR hash/size and references are signed).
    for info in output.glob('*.narinfo'):
        contents = info.read_text()
        match = re.search(r'^URL: (nar/[^\n]+)$', contents, re.M)
        if not match:
            raise ValueError(f'Unexpected NAR URL in {info.name}')
        relative = Path(match[1])
        name = relative.name
        source, destination = output / relative, output / name
        if source.exists():
            source.rename(destination)
        if not destination.is_file():
            raise ValueError('Missing compressed store object')
        info.write_text(contents[:match.start(1)] + name + contents[match.end(1):])
        if '\nSig: ' + public.split(':', 1)[0] + ':' not in contents:
            raise ValueError(f'Unsigned store object: {info.name}')
    (output / 'nar').rmdir()
    script = (ROOT / 'install.sh').read_text().replace("PUBLIC_KEY='@CACHE_PUBLIC_KEY@'", f"PUBLIC_KEY='{public}'")
    script = script.replace("VERSION='@RELEASE_VERSION@'", f"VERSION='{version}'")
    (output / 'install.sh').write_text(script)
    (output / 'install.sh').chmod(0o755)
    system = subprocess.check_output(['nix', '--extra-experimental-features', 'nix-command',
                                      'eval', '--raw', '--impure', '--expr', 'builtins.currentSystem']).decode()
    (output / f'release-{system}.txt').write_text(f'nixos-toolkit-release-v1\n{version}\n{system}\n{installer}\n')
    (output / 'cache-public-key').write_text(public + '\n')
    (output / 'INSTALLING.md').write_text((ROOT / 'INSTALLING.md').read_text())
    files = sorted(p for p in output.iterdir() if p.is_file())
    if len(files) + 2 > 1000:
        raise ValueError('Too many assets for a GitHub release; use an HTTP cache host')
    if any(p.stat().st_size >= 2 * 1024 ** 3 for p in files):
        raise ValueError('An asset exceeds GitHub’s 2 GiB limit')
    report = {'version': version, 'system': system, 'installer': str(installer),
              'store_paths': len(list(output.glob('*.narinfo'))),
              'compressed_bytes': sum(p.stat().st_size for p in output.glob('*.nar.xz')),
              'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip()}
    (output / 'build-info.json').write_text(json.dumps(report, indent=2) + '\n')
    files.append(output / 'build-info.json')
    checksums = []
    for path in sorted(files):
        with path.open('rb') as handle:
            checksums.append(f'{hashlib.file_digest(handle, "sha256").hexdigest()}  {path.name}\n')
    (output / 'SHA256SUMS').write_text(''.join(checksums))
    print(json.dumps(report, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--version', required=True)
    parser.add_argument('--signing-key', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--installer', type=Path, help='Already built .#installer output')
    args = parser.parse_args()
    installer = args.installer
    if installer is None:
        installer = Path(subprocess.check_output(['nix', '--extra-experimental-features', 'nix-command flakes',
                                                 'build', '.#installer', '--no-link', '--print-out-paths'], cwd=ROOT, text=True).strip())
    prepare(args.output, installer, args.version, args.signing_key)


if __name__ == '__main__':
    main()
