#!/usr/bin/env python3
"""Relocate a prebuilt Flutter bundle to the Flatpak runtime, including Nix builds.

The resulting Flatpak must pass diagnostics inside its runtime. Relocating ELF
paths does not make an arbitrary libc version compatible with that runtime.
"""
import os
from pathlib import Path
import stat
import subprocess
import sys


def relocate(bundle, patchelf='patchelf'):
    bundle = Path(bundle).resolve(strict=True)
    interpreters = {62: '/lib64/ld-linux-x86-64.so.2', 183: '/lib/ld-linux-aarch64.so.1'}
    for path in sorted(bundle.rglob('*')):
        if not path.is_file() or path.is_symlink():
            continue
        with path.open('rb') as stream:
            header = stream.read(20)
        if header[:4] != b'\x7fELF':
            continue
        byteorder = 'little' if header[5] == 1 else 'big'
        machine = int.from_bytes(header[18:20], byteorder)
        if machine not in interpreters:
            raise ValueError(f'Unsupported ELF architecture {machine}: {path}')
        path.chmod(path.stat().st_mode | stat.S_IWUSR)
        library_dir = os.path.relpath(bundle / 'lib', path.parent)
        rpath = '$ORIGIN' if library_dir == '.' else f'$ORIGIN/{library_dir}'
        subprocess.run([patchelf, '--set-rpath', rpath, str(path)], check=True)
        interpreter = subprocess.run([patchelf, '--print-interpreter', str(path)], capture_output=True, text=True)
        if interpreter.returncode == 0:
            subprocess.run([patchelf, '--set-interpreter', interpreters[machine], str(path)], check=True)
        needed = subprocess.check_output([patchelf, '--print-needed', str(path)], text=True)
        for name in needed.splitlines():
            if name.startswith('/nix/store/'):
                subprocess.run([patchelf, '--replace-needed', name, Path(name).name, str(path)], check=True)


if __name__ == '__main__':
    relocate(sys.argv[1])
