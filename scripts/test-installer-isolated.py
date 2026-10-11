#!/usr/bin/env python3
"""Exercise the real installer in a private NixOS filesystem and fake activation.

No host configuration is writable. These tests cover transactional behavior;
real package signatures, downloads and activation require the release/VM checks.
"""
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('installer', REPO / 'packaging/nix/install.py')
installer = importlib.util.module_from_spec(spec)
spec.loader.exec_module(installer)
SYSTEM = str(Path('/run/current-system').resolve()) if Path('/run/current-system').exists() else str(next(Path('/nix/store').glob('*-source')))
CANDIDATE = str(next(p for p in Path('/nix/store').glob('*-source') if str(p) != SYSTEM))


class ConfigTests(unittest.TestCase):
    def test_comments_strings_and_nested_imports_are_not_module_imports(self):
        source = '''# imports = [ wrong ];
{ config, pkgs, ... }: {
  text = "imports = [ wrong ]; ${toString { imports = []; }}";
  shell = ''echo "${if true then "imports" else "other"}"; '';
  nested = { imports = [ ./other.nix ]; };
  imports = [ ./hardware-configuration.nix ];
}'''
        changed = installer.edit_configuration(source)
        self.assertIn('imports = [\n    # BEGIN NIXOS TOOLKIT', changed)
        self.assertIn('nested = { imports = [ ./other.nix ]; };', changed)
        self.assertEqual(installer.edit_configuration(changed), changed)
        removed = installer.edit_configuration(changed, uninstall=True)
        self.assertNotIn('/etc/nixos/nixos-toolkit-install.nix', removed)
        self.assertIn('/etc/nixos/nixos-toolkit/state/selected.nix', removed)
        self.assertIn('./hardware-configuration.nix', removed)

    def test_missing_imports_and_existing_selected(self):
        changed = installer.edit_configuration('{ ... }: { networking.hostName = "test"; }')
        self.assertIn('imports = [', changed)
        source = '{ imports = [ ./nixos-toolkit/state/selected.nix ]; }'
        self.assertEqual(installer.edit_configuration(source).count('state/selected.nix'), 1)

    def test_computed_modules_and_edited_markers_fail_closed(self):
        for source in ['let x = {}; in x', '{ ... }: { imports = makeImports []; }',
                       '{ ... }: let x = []; in { imports = x; }']:
            with self.subTest(source=source), self.assertRaises(RuntimeError):
                installer.edit_configuration(source)
        changed = installer.edit_configuration('{ ... }: {}')
        with self.assertRaises(RuntimeError):
            installer.edit_configuration(changed.replace('/etc/nixos/nixos-toolkit-install.nix', './manual.nix'))


class BootstrapTests(unittest.TestCase):
    def invoke(self, manifest=None, *args, unavailable=False):
        with tempfile.TemporaryDirectory(prefix='toolkit-bootstrap-test-') as folder:
            root = Path(folder)
            (root / 'etc').mkdir()
            (root / 'etc/NIXOS').touch()
            (root / 'bin').mkdir()
            valid = 'nixos-toolkit-release-v1\nv0.1.0\nx86_64-linux\n/nix/store/' + 'a' * 32 + '-nixos-toolkit-installer-0.1.0\n'
            (root / 'manifest').write_text(valid if manifest is None else manifest)
            mock = root / 'bin/mock'
            mock.write_text(f'#!{sys.executable}\n' + '''
import json, os, pathlib, sys
name = pathlib.Path(sys.argv[0]).name
with open('/fixture/commands.jsonl', 'a') as out:
    out.write(json.dumps([name, *sys.argv[1:]]) + '\\n')
if name == 'curl':
    if os.environ.get('UNAVAILABLE') == '1': sys.exit(22)
    pathlib.Path(sys.argv[sys.argv.index('-o') + 1]).write_bytes(pathlib.Path('/fixture/manifest').read_bytes())
''')
            mock.chmod(0o755)
            for name in ['curl', 'sudo', 'nix-store']:
                (root / 'bin' / name).symlink_to('mock')
            command = ['bwrap', '--unshare-all', '--uid', '1000', '--gid', '1000', '--die-with-parent',
                       '--ro-bind', '/nix/store', '/nix/store', '--ro-bind', str(REPO), '/app',
                       '--bind', str(root), '/fixture', '--bind', str(root / 'etc'), '/etc',
                       '--dev', '/dev', '--proc', '/proc', '--tmpfs', '/tmp', '--chdir', '/fixture',
                       '--setenv', 'TMPDIR', '/tmp',
                       '--setenv', 'PATH', '/fixture/bin:' + os.environ['PATH'],
                       '--setenv', 'UNAVAILABLE', '1' if unavailable else '0',
                       shutil.which('bash'), '/app/install.sh', '--no-launch', *args]
            result = subprocess.run(command, text=True, capture_output=True, timeout=20)
            log = root / 'commands.jsonl'
            commands = [json.loads(s) for s in log.read_text().splitlines()] if log.exists() else []
            return result, commands

    def test_valid_download_is_signed_and_disallows_build_fallback(self):
        result, commands = self.invoke()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        download = next(c for c in commands if c[:3] == ['sudo', 'nix-store', '--realise'])
        for option, expected in [('require-sigs', 'true'), ('max-jobs', '0'), ('builders', '')]:
            self.assertEqual(download[download.index(option) + 1], expected)
        self.assertIn('https://cache.nixos.org https://github.com/goshitsarch-eng/Gosh-NixOS-Manager/releases/download/v0.1.0', download)
        self.assertEqual(commands[-1][-1], 'install')

    def test_malformed_manifest_never_reaches_sudo(self):
        result, commands = self.invoke('$(touch /tmp/not-allowed)\n')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(any(c[0] == 'sudo' for c in commands))

    def test_missing_release_does_not_authenticate_or_change_system(self):
        result, commands = self.invoke(unavailable=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(any(c[0] == 'sudo' for c in commands))

    def test_release_version_mismatch_never_reaches_sudo(self):
        result, commands = self.invoke(None, '--version', 'v99.0.0')
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(any(c[0] == 'sudo' for c in commands))


class TransactionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='toolkit-installer-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in ['etc/nixos', 'run/lock', 'profiles', 'gcroots', 'var', 'bin', 'old/bin',
                     'candidate/bin', 'candidate/sw/bin', 'gui/bin', 'helper/bin', 'pkg/share/nixos-toolkit-installer']:
            (self.root / name).mkdir(parents=True)
        (self.root / 'etc/NIXOS').touch()
        self.config = self.root / 'etc/nixos/configuration.nix'
        self.original = '{ ... }: { imports = [ ]; networking.hostName = "original"; }\n'
        self.config.write_text(self.original)
        (self.root / 'run/current-system').symlink_to(SYSTEM)
        (self.root / 'profiles/system').symlink_to(SYSTEM)
        self.data = self.root / 'pkg/share/nixos-toolkit-installer'
        shutil.copy2(REPO / 'packaging/nix/install.py', self.data / 'install.py')
        (self.data / 'packages.json').write_text(json.dumps({'gui': '/fixture/gui', 'helper': '/fixture/helper', 'version': '0.1.0'}))
        for name, package in [('nixos-toolkit', 'gui'), ('nixos-toolkit-helper', 'helper')]:
            (self.root / package / 'bin' / name).write_text('fixture executable')
            (self.root / 'candidate/sw/bin' / name).symlink_to('/fixture/' + package + '/bin/' + name)
        mock = self.root / 'bin/mock'
        mock.write_text(f'#!{sys.executable}\n' + '''
import json, os, pathlib, sys
name = pathlib.Path(sys.argv[0]).name
with open('/fixture/commands.jsonl', 'a') as out:
    out.write(json.dumps([sys.argv[0], *sys.argv[1:]]) + '\\n')
if name == 'nixos-rebuild':
    if os.environ.get('FAIL_BUILD'):
        sys.exit(42)
    pathlib.Path('result').symlink_to(os.environ['CANDIDATE'])
    if os.environ.get('EXTERNAL_EDIT'):
        with open('/etc/nixos/configuration.nix', 'a') as out: out.write('# external editor\\n')
elif name == 'nix-env':
    profile = pathlib.Path('/nix/var/nix/profiles/system')
    profile.unlink()
    profile.symlink_to(sys.argv[-1])
elif name == 'switch-to-configuration':
    if sys.argv[0].startswith(os.environ['CANDIDATE']) and os.environ.get('FAIL_ACTIVATION'):
        sys.exit(43)
    active = pathlib.Path('/run/current-system')
    active.unlink()
    active.symlink_to(str(pathlib.Path(sys.argv[0]).parent.parent))
''')
        mock.chmod(0o755)
        for name in ['nixos-rebuild', 'nix-env', 'git']:
            (self.root / 'bin' / name).symlink_to('mock')
        for directory in ['old', 'candidate']:
            (self.root / directory / 'bin/switch-to-configuration').symlink_to('/fixture/bin/mock')

    def invoke(self, action='install', *args, **env):
        command = ['bwrap', '--unshare-all', '--uid', '0', '--gid', '0', '--die-with-parent',
                   '--ro-bind', '/nix/store', '/nix/store', '--bind', str(self.root), '/fixture',
                   '--bind', str(self.root / 'etc'), '/etc', '--bind', str(self.root / 'run'), '/run',
                   '--bind', str(self.root / 'profiles'), '/nix/var/nix/profiles',
                   '--bind', str(self.root / 'gcroots'), '/nix/var/nix/gcroots',
                   '--bind', str(self.root / 'var'), '/var',
                   '--ro-bind', str(self.root / 'old'), SYSTEM,
                   '--ro-bind', str(self.root / 'candidate'), CANDIDATE,
                   '--dev', '/dev', '--proc', '/proc', '--tmpfs', '/tmp', '--chdir', '/fixture',
                   '--setenv', 'PATH', '/fixture/bin', '--setenv', 'CANDIDATE', CANDIDATE]
        for key, value in env.items():
            command.extend(['--setenv', key, str(value)])
        return subprocess.run([*command, sys.executable, '/fixture/pkg/share/nixos-toolkit-installer/install.py', action, *args],
                              text=True, capture_output=True, timeout=20)

    def success(self, result):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def commands(self):
        log = self.root / 'commands.jsonl'
        return [json.loads(line) for line in log.read_text().splitlines()] if log.exists() else []

    def test_install_repeat_and_uninstall_preserve_user_edits_and_selections(self):
        self.success(self.invoke())
        saved = self.config.read_text()
        self.assertIn('BEGIN NIXOS TOOLKIT', saved)
        self.assertTrue((self.root / 'etc/nixos/nixos-toolkit-install.nix').is_file())
        self.success(self.invoke())
        self.assertEqual(self.config.read_text(), saved)
        self.config.write_text(saved.replace('original', 'edited-after-install'))
        self.success(self.invoke('uninstall'))
        self.assertIn('edited-after-install', self.config.read_text())
        self.assertIn('nixos-toolkit/state/selected.nix', self.config.read_text())
        self.assertNotIn('BEGIN NIXOS TOOLKIT', self.config.read_text())
        self.assertFalse((self.root / 'etc/nixos/nixos-toolkit-install.nix').exists())
        self.assertTrue((self.root / 'etc/nixos/nixos-toolkit/state/selected.nix').exists())

    def test_failed_build_restores_files_without_activation(self):
        result = self.invoke(FAIL_BUILD=1)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.config.read_text(), self.original)
        self.assertFalse((self.root / 'etc/nixos/nixos-toolkit-install.nix').exists())
        self.assertFalse((self.root / 'etc/nixos/nixos-toolkit/state/selected.nix').exists())
        self.assertFalse(any('switch-to-configuration' in command[0] for command in self.commands()))
        self.assertFalse((self.root / 'var/lib/nixos-toolkit-installer/pending.json').exists())

    def test_failed_activation_restores_running_and_boot_system(self):
        result = self.invoke(FAIL_ACTIVATION=1)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(self.config.read_text(), self.original)
        self.assertEqual(os.readlink(self.root / 'run/current-system'), SYSTEM)
        self.assertEqual(os.readlink(self.root / 'profiles/system'), SYSTEM)
        self.assertTrue(any(command == [SYSTEM + '/bin/switch-to-configuration', 'switch'] for command in self.commands()))

    def test_existing_module_is_never_overwritten(self):
        path = self.root / 'etc/nixos/nixos-toolkit-install.nix'
        path.write_text('user-owned')
        self.assertNotEqual(self.invoke().returncode, 0)
        self.assertEqual(path.read_text(), 'user-owned')
        self.assertEqual(self.config.read_text(), self.original)
        self.assertEqual(self.commands(), [])

    def test_symlinked_configuration_is_not_followed(self):
        self.config.rename(self.config.with_suffix('.original'))
        self.config.symlink_to('configuration.original')
        self.assertNotEqual(self.invoke().returncode, 0)
        self.assertTrue(self.config.is_symlink())
        self.assertEqual(self.config.read_text(), self.original)

    def test_flake_requires_explicit_target_and_preserves_lock(self):
        (self.root / 'etc/nixos/flake.nix').write_text('{}')
        lock = self.root / 'etc/nixos/flake.lock'
        lock.write_text('unchanged')
        self.assertNotEqual(self.invoke().returncode, 0)
        self.assertEqual(self.commands(), [])
        self.success(self.invoke('install', '--flake', '/etc/nixos#test'))
        self.assertEqual(lock.read_text(), 'unchanged')
        self.assertIn('--flake', self.commands()[0])

    def test_pending_helper_recovery_blocks_install(self):
        (self.root / 'var/lib/nixos-toolkit/recovery').mkdir(parents=True)
        self.assertNotEqual(self.invoke().returncode, 0)
        self.assertEqual(self.config.read_text(), self.original)
        self.assertEqual(self.commands(), [])

    def test_private_backup_and_edited_module_protection(self):
        self.success(self.invoke())
        backups = list((self.root / 'var/lib/nixos-toolkit-installer/backups').glob('*.json'))
        self.assertTrue(backups)
        self.assertTrue(all(path.stat().st_mode & 0o077 == 0 for path in backups))
        path = self.root / 'etc/nixos/nixos-toolkit-install.nix'
        path.write_text(path.read_text() + '# user change\n')
        self.assertNotEqual(self.invoke().returncode, 0)
        self.assertTrue(path.read_text().endswith('# user change\n'))

    def simulate_interruption(self):
        self.success(self.invoke())
        backup = next((self.root / 'var/lib/nixos-toolkit-installer/backups').glob('*.json'))
        record = json.loads(backup.read_text())
        record['activation_started'] = True
        pending = self.root / 'var/lib/nixos-toolkit-installer/pending.json'
        pending.write_text(json.dumps(record))
        pending.chmod(0o600)
        return pending

    def test_interrupted_install_requires_recovery_and_restores_previous_system(self):
        pending = self.simulate_interruption()
        self.assertNotEqual(self.invoke().returncode, 0)
        self.success(self.invoke('recover'))
        self.assertEqual(self.config.read_text(), self.original)
        self.assertEqual(os.readlink(self.root / 'run/current-system'), SYSTEM)
        self.assertFalse(pending.exists())

    def test_recovery_preserves_external_edits_and_retains_backup(self):
        pending = self.simulate_interruption()
        self.config.write_text(self.config.read_text() + '# changed by editor\n')
        self.assertNotEqual(self.invoke('recover').returncode, 0)
        self.assertTrue(self.config.read_text().endswith('# changed by editor\n'))
        self.assertTrue(pending.exists())

    def test_missing_package_in_build_is_rejected_before_activation(self):
        (self.root / 'candidate/sw/bin/nixos-toolkit-helper').unlink()
        self.assertNotEqual(self.invoke().returncode, 0)
        self.assertEqual(self.config.read_text(), self.original)
        self.assertFalse(any('switch-to-configuration' in command[0] for command in self.commands()))

    def test_running_helper_prevents_installer_mutations(self):
        import fcntl
        with (self.root / 'run/lock/nixos-toolkit.lock').open('w') as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            self.assertNotEqual(self.invoke().returncode, 0)
        self.assertEqual(self.config.read_text(), self.original)
        self.assertEqual(self.commands(), [])

    def test_configuration_changed_during_build_is_not_activated_or_overwritten(self):
        result = self.invoke(EXTERNAL_EDIT=1)
        self.assertNotEqual(result.returncode, 0)
        self.assertTrue(self.config.read_text().endswith('# external editor\n'))
        self.assertFalse(any('switch-to-configuration' in command[0] for command in self.commands()))
        self.assertTrue((self.root / 'var/lib/nixos-toolkit-installer/pending.json').exists())


if __name__ == '__main__':
    unittest.main(verbosity=2)
