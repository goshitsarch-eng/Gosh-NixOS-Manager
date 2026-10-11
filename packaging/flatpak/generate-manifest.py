#!/usr/bin/env python3
"""Package a checksummed Flutter release bundle; no dependency fetch in sandbox."""
import hashlib
import json
import pathlib
import subprocess
import sys
import tarfile
import tempfile

archive = pathlib.Path(sys.argv[1]).resolve(strict=True)
output = pathlib.Path(sys.argv[2]).resolve()
output.parent.mkdir(parents=True, exist_ok=True)
# Keep the input archive intact. Nix-built ELF files contain store-specific
# interpreter/RPATH entries, which cannot resolve inside a Flatpak runtime.
with tempfile.TemporaryDirectory(prefix='toolkit-flatpak-') as folder:
    with tarfile.open(archive) as source:
        source.extractall(folder, filter='data')
    entries = list(pathlib.Path(folder).iterdir())
    if len(entries) != 1 or not (entries[0] / 'nixos-toolkit').is_file():
        sys.exit('Expected one Flutter bundle directory in the release archive')
    subprocess.run([sys.executable, str(pathlib.Path(__file__).parent / 'relocate-elf.py'), str(entries[0])], check=True)
    archive = output.with_suffix('.bundle.tar.gz')
    with tarfile.open(archive, 'w:gz') as portable:
        portable.add(entries[0], arcname=entries[0].name)
app_id = 'io.github.goshitsarch_eng.NixosToolkit'
manifest = {
    'id': app_id, 'runtime': 'org.freedesktop.Platform', 'runtime-version': '25.08',
    'sdk': 'org.freedesktop.Sdk', 'command': 'nixos-toolkit',
    'finish-args': [
        '--socket=wayland', '--socket=fallback-x11', '--share=ipc', '--device=dri',
        '--talk-name=org.freedesktop.Flatpak',
        '--env=NIXOS_TOOLKIT_TEMPLATES_DIR=/app/lib/nixos-toolkit/data/templates',
    ],
    'modules': [{
        'name': 'nixos-toolkit', 'buildsystem': 'simple',
        'build-commands': [
            'mkdir -p /app/lib/nixos-toolkit',
            'cp -a nixos-toolkit lib data /app/lib/nixos-toolkit/',
            'install -Dm755 flatpak-launcher /app/bin/nixos-toolkit',
            'cp -a share /app/',
            f'install -Dm644 LICENSE /app/share/licenses/{app_id}/LICENSE',
        ],
        'sources': [
            {'type': 'archive', 'path': str(archive), 'sha256': hashlib.sha256(archive.read_bytes()).hexdigest()},
            {'type': 'file', 'path': str(pathlib.Path(__file__).parent / 'launcher'), 'dest-filename': 'flatpak-launcher'},
        ],
    }],
}
output.parent.mkdir(parents=True, exist_ok=True)
output.write_text(json.dumps(manifest, indent=2) + '\n')
