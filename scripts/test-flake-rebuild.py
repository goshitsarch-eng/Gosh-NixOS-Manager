#!/usr/bin/env python3
"""Real Nix dry-builds of plain/Git flakes in a private configuration namespace.

Usage: python3 scripts/test-flake-rebuild.py /path/to/nixpkgs
Requires built helper/core, bubblewrap, Git, nixos-rebuild and the Nix daemon.
No host configuration, profile, activation, or Git index is changed.
"""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('helper_qa', ROOT / 'scripts/test-helper-isolated.py')
helper_qa = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper_qa)
nixpkgs = Path(sys.argv[1]).resolve(strict=True)
evidence = ROOT / 'dist/qa-flake-rebuild'
evidence.mkdir(parents=True, exist_ok=True)

for git in (False, True):
    with tempfile.TemporaryDirectory(prefix='toolkit-flake-qa-') as temporary:
        folder = Path(temporary)
        for name in ['etc/nixos/nixos-toolkit/state', 'etc/nix', 'lock', 'var', 'gcroots']:
            (folder / name).mkdir(parents=True)
        (folder / 'etc/NIXOS').touch()
        (folder / 'etc/nix/nix.conf').write_text('experimental-features = nix-command flakes\n')
        (folder / 'etc/passwd').write_text(f'qa:x:{os.getuid()}:{os.getgid()}::/tmp:/bin/sh\n')
        (folder / 'etc/group').write_text(f'qa:x:{os.getgid()}:\n')
        config = folder / 'etc/nixos'
        selected = config / 'nixos-toolkit/state/selected.nix'
        selected.write_text('{ imports = []; }')
        (config / 'flake.nix').write_text('''{
          inputs.nixpkgs.url = "path:''' + str(nixpkgs) + '''";
          outputs = { self, nixpkgs }: {
            nixosConfigurations.toolkit-qa = nixpkgs.lib.nixosSystem {
              system = "x86_64-linux";
              modules = [ ./nixos-toolkit/state/selected.nix ({ ... }: {
                system.stateVersion = "25.05"; boot.isContainer = true;
                networking.useHostResolvConf = false;
              }) ];
            };
          };
        }''')
        # An ignored local file must never be swept into a Git flake or index.
        (config / '.gitignore').write_text('private-local-file\n')
        (config / 'private-local-file').write_text('local fixture, never stage')
        subprocess.run(['nix', '--extra-experimental-features', 'nix-command flakes',
                        'flake', 'lock', 'path:' + str(config)], check=True,
                       capture_output=True)
        if git:
            subprocess.run(['git', '-C', str(config), 'init', '-q'], check=True)
            subprocess.run(['git', '-C', str(config), 'add', 'flake.nix', 'flake.lock', '.gitignore',
                            'nixos-toolkit/state/selected.nix'], check=True)
            index = subprocess.check_output(['git', '-C', str(config), 'ls-files', '--stage', '-z'])
        command = [
            shutil.which('bwrap'), '--unshare-all', '--share-net', '--die-with-parent',
            '--hostname', 'toolkit-qa', '--ro-bind', '/', '/',
            '--bind', str(folder / 'etc'), '/etc',
            '--bind', str(folder / 'lock'), '/run/lock',
            '--bind', str(folder / 'var'), '/var',
            '--bind', str(folder / 'gcroots'), '/nix/var/nix/gcroots',
            '--dev', '/dev', '--proc', '/proc', '--tmpfs', '/tmp',
            '--setenv', 'TMPDIR', '/tmp', '--setenv', 'XDG_CACHE_HOME', '/tmp/cache',
            '--setenv', 'NIX_REMOTE', 'daemon', '--chdir', '/tmp',
            '--', str(ROOT / 'target/debug/nixos-toolkit-helper'),
        ]
        state = helper_qa.bootstrap()['defaults']
        state.pop('last_applied', None)
        # New snippets, a profile and a bundle all need to survive Git filtering.
        state.update(hostname='toolkit-qa', selected_profile='xfce',
                     enabled_bundles=['devtools'], custom_packages=['hello'])
        request = {'type': 'Apply', 'payload': dict(state, rebuild_type='DryBuild')}
        result = subprocess.run(command, input=json.dumps(request) + '\n', text=True,
                                capture_output=True, timeout=300)
        name = 'git' if git else 'plain'
        (evidence / f'{name}.log').write_text(result.stdout + result.stderr)
        assert result.returncode == 0, result.stderr
        reply = json.loads(result.stdout.splitlines()[-1])
        assert reply['type'] == 'ApplyComplete' and reply['payload']['success'], reply
        assert selected.read_text() == '{ imports = []; }'
        assert not (folder / 'var/lib/nixos-toolkit/recovery').exists()
        if git:
            # Git may refresh stat-cache metadata; staged paths/blobs must stay unchanged.
            assert subprocess.check_output(['git', '-C', str(config), 'ls-files', '--stage', '-z']) == index
        print(f'PASS {name} flake: real dry-build, managed files restored, Git index preserved', flush=True)
