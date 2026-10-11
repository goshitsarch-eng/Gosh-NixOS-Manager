#!/usr/bin/env python3
"""Privileged, transactional integration of already downloaded Nix packages.

This program lives in the signed Nix store. It never downloads or compiles the
app. Configuration contents and rollback records remain root-readable only.
"""
import argparse
import base64
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import tempfile
import time

CONFIG = Path('/etc/nixos/configuration.nix')
MODULE = Path('/etc/nixos/nixos-toolkit-install.nix')
SELECTED = Path('/etc/nixos/nixos-toolkit/state/selected.nix')
STATE = Path('/var/lib/nixos-toolkit-installer')
ROOTS = Path('/nix/var/nix/gcroots/nixos-toolkit-installer')
PENDING = STATE / 'pending.json'
RECEIPT = STATE / 'installed.json'
DATA = Path(__file__).resolve().parent
PACKAGE = DATA.parent.parent
BEGIN = '# BEGIN NIXOS TOOLKIT INSTALLER\n'
END = '# END NIXOS TOOLKIT INSTALLER\n'
MODULE_MARKER = '# Installed by the NixOS Toolkit prebuilt installer.\n'


def run(argv, **kwargs):
    return subprocess.run([str(x) for x in argv], check=True, **kwargs)


def digest(data):
    return hashlib.sha256(data).hexdigest() if data is not None else None


def regular(path):
    # Reject symlinks instead of modifying the target of somebody else's config.
    for parent in [path, *path.parents]:
        if parent.is_symlink():
            raise RuntimeError(f'{parent} is a symlink; integrate the NixOS module manually.')
    if path.exists() and not path.is_file():
        raise RuntimeError(f'{path} is not a regular file.')


def atomic(path, content, mode=0o600, uid=0, gid=0):
    regular(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(prefix='.toolkit-', dir=path.parent)
    try:
        with os.fdopen(fd, 'wb') as output:
            os.fchmod(output.fileno(), mode)
            os.fchown(output.fileno(), uid, gid)
            output.write(content)
            output.flush()
            os.fsync(output.fileno())
        os.replace(name, path)
        directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        if os.path.exists(name):
            os.unlink(name)


def save_json(path, data):
    atomic(path, (json.dumps(data, indent=2) + '\n').encode())


def tokens(source):
    """Lex enough Nix to locate a literal top-level imports list, never evaluate it."""
    result = []
    length = len(source)

    def string(i, quote):
        i += len(quote)
        while i < length:
            if quote == '"' and source[i] == '\\':
                i += 2
            elif quote == "''" and source.startswith(("'''", "''$", "''\\"), i):
                i += 3
            elif source.startswith(quote, i):
                return i + len(quote)
            elif source.startswith('${', i):
                i = expression(i + 2)
            else:
                i += 1
        raise RuntimeError('Unterminated Nix string; configuration was not changed.')

    def expression(i):
        depth = 1
        while i < length:
            if source.startswith('/*', i):
                end = source.find('*/', i + 2)
                if end < 0:
                    raise RuntimeError('Unterminated Nix comment.')
                i = end + 2
            elif source[i] == '#':
                end = source.find('\n', i)
                i = length if end < 0 else end + 1
            elif source[i] == '"' or source.startswith("''", i):
                i = string(i, '"' if source[i] == '"' else "''")
            else:
                if source[i] == '{':
                    depth += 1
                elif source[i] == '}':
                    depth -= 1
                    if depth == 0:
                        return i + 1
                i += 1
        raise RuntimeError('Unterminated Nix interpolation.')

    i = 0
    while i < length:
        start = i
        if source[i].isspace():
            i += 1
        elif source[i] == '#':
            end = source.find('\n', i)
            i = length if end < 0 else end + 1
        elif source.startswith('/*', i):
            end = source.find('*/', i + 2)
            if end < 0:
                raise RuntimeError('Unterminated Nix comment.')
            i = end + 2
        elif source[i] == '"' or source.startswith("''", i):
            i = string(i, '"' if source[i] == '"' else "''")
            result.append(('<string>', start, i))
        else:
            match = re.match(r"[A-Za-z_][A-Za-z0-9_'/-]*|[/~.][A-Za-z0-9_./+-]+", source[i:])
            value = match.group() if match else source[i]
            i += len(value)
            result.append((value, start, i))
    return result


def edit_configuration(source, uninstall=False):
    if BEGIN in source or END in source:
        if source.count(BEGIN) != 1 or source.count(END) != 1:
            raise RuntimeError('Installer import markers were edited; repair them before continuing.')
        start, end = source.index(BEGIN), source.index(END) + len(END)
        owned = source[start:end]
        expected = BEGIN + '    /etc/nixos/nixos-toolkit-install.nix\n    ' + END
        if owned != expected:
            raise RuntimeError('Installer import block was edited; refusing to overwrite it.')
        return source[:start] + source[end:] if uninstall else source
    if uninstall:
        raise RuntimeError('Installer import block is missing; remove the module import manually.')
    ts = tokens(source)
    values = [t[0] for t in ts]
    if not values or values[0] != '{':
        raise RuntimeError('Automatic setup needs a conventional configuration.nix module. See INSTALLING.md.')
    depth = 0
    first_end = None
    for i, value in enumerate(values):
        depth += (value == '{') - (value == '}')
        if depth == 0:
            first_end = i
            break
    body = 0
    if first_end is not None and values[first_end + 1:first_end + 3] == [':', '{']:
        body = first_end + 2
    elif first_end != len(ts) - 1:
        raise RuntimeError('Complex Nix module expression; use the manual integration instructions.')
    depth = 0
    insertion = None
    for i in range(body, len(ts)):
        value = values[i]
        if depth == 1 and value == 'imports':
            if values[i + 1:i + 3] != ['=', '[']:
                raise RuntimeError('Computed imports need manual integration; configuration was not changed.')
            insertion = ts[i + 2][2]
            break
        if value in ('{', '[', '('):
            depth += 1
        elif value in ('}', ']', ')'):
            depth -= 1
    block = '\n    ' + BEGIN + '    /etc/nixos/nixos-toolkit-install.nix\n    ' + END
    # Keep applied settings working if the GUI/helper is later uninstalled.
    if not any('nixos-toolkit/state/selected.nix' in v for v in values):
        block += '    # Toolkit selections: retained when uninstalling the application.\n'
        block += '    /etc/nixos/nixos-toolkit/state/selected.nix\n'
    if insertion is None:
        insertion = ts[body][2]
        block = '\n  imports = [' + block + '  ];\n'
    return source[:insertion] + block + source[insertion:]


def module(packages):
    compat = DATA / 'nix/modules/activation-reload-compat.nix'
    return (MODULE_MARKER + f'''{{ config, lib, ... }}:
{{
  imports = [ {compat} ];
  programs.nixos-toolkit.serializeDbusReload = lib.mkDefault (config.system.nixos.release == "25.05");
  security.polkit.enable = true;
  environment.systemPackages = [
    (builtins.storePath "{packages['gui']}")
    (builtins.storePath "{packages['helper']}")
  ];
}}
''').encode()


def roots(record):
    ROOTS.mkdir(parents=True, exist_ok=True)
    if record['previous_installer']:
        previous = ROOTS / 'previous-installer'
        previous.unlink(missing_ok=True)
        previous.symlink_to(record['previous_installer'])
    for name, value in [('installer', str(PACKAGE)), ('previous-running', record['running']),
                        ('previous-boot', record['boot'])]:
        path = ROOTS / name
        temporary = ROOTS / (name + '.new')
        temporary.unlink(missing_ok=True)
        temporary.symlink_to(value)
        os.replace(temporary, path)


def restore(record):
    # Do not clobber an editor's changes made after this installer wrote a file.
    for item in record['files']:
        path = Path(item['path'])
        regular(path)
        current = path.read_bytes() if path.exists() else None
        before = base64.b64decode(item['before']) if item['before'] is not None else None
        if digest(current) not in (item['after_hash'], digest(before)):
            raise RuntimeError(f'{path} changed outside the installer; backups retained in {STATE}.')
    for item in record['files']:
        path = Path(item['path'])
        if item['before'] is None:
            path.unlink(missing_ok=True)
        else:
            atomic(path, base64.b64decode(item['before']), item['mode'], item['uid'], item['gid'])
    if record['activation_started']:
        run(['nix-env', '--profile', '/nix/var/nix/profiles/system', '--set', record['boot']])
        if record['boot'] == record['running']:
            run([record['running'] + '/bin/switch-to-configuration', 'switch'])
        else:
            run([record['boot'] + '/bin/switch-to-configuration', 'boot'])
            run([record['running'] + '/bin/switch-to-configuration', 'test'])
    for path in record.get('git_added', []):
        run(['git', '-c', 'safe.directory=/etc/nixos', '-C', '/etc/nixos', 'reset', '--quiet', '--', path])
    (ROOTS / 'installer').unlink(missing_ok=True)
    if record['previous_installer']:
        (ROOTS / 'installer').symlink_to(record['previous_installer'])
    PENDING.unlink(missing_ok=True)
    for name in ('previous-running', 'previous-boot', 'previous-installer'):
        (ROOTS / name).unlink(missing_ok=True)
    print('Restored the previous configuration. Private backups were retained.', flush=True)


def rebuild_args(flake):
    if flake:
        if not re.fullmatch(r'/etc/nixos#[A-Za-z0-9_.-]+', flake):
            raise RuntimeError('Use an explicit local target: --flake /etc/nixos#HOST')
        return ['--flake', flake, '--no-write-lock-file', '--no-update-lock-file',
                '--option', 'extra-experimental-features', 'nix-command flakes']
    if Path('/etc/nixos/flake.nix').exists():
        raise RuntimeError('This host uses flakes; specify --flake /etc/nixos#HOST.')
    # Pin the standard configuration rather than inheriting a caller's NIX_PATH target.
    return ['-I', 'nixos-config=/etc/nixos/configuration.nix']


def transact(action, flake):
    args = rebuild_args(flake)
    packages = json.loads((DATA / 'packages.json').read_text())
    regular(CONFIG)
    original = CONFIG.read_text()
    for path in (MODULE, SELECTED, RECEIPT):
        regular(path)
    installed = json.loads(RECEIPT.read_text()) if RECEIPT.exists() else None
    if action == 'uninstall' and installed is None:
        raise RuntimeError('No installation managed by this installer was found.')
    if MODULE.exists() and (installed is None or digest(MODULE.read_bytes()) != installed['module_hash']):
        raise RuntimeError(f'{MODULE} has manual changes or another owner; move those changes before installing.')
    changes = {CONFIG: edit_configuration(original, action == 'uninstall').encode(),
               MODULE: None if action == 'uninstall' else module(packages)}
    if action == 'install' and not SELECTED.exists():
        changes[SELECTED] = b'# NixOS Toolkit: no selections yet.\n{ ... }: { }\n'
    receipt = None if action == 'uninstall' else json.dumps({
        'version': packages['version'], 'module_hash': digest(changes[MODULE]),
        'flake': flake, 'installer': str(PACKAGE),
    }, indent=2).encode()
    changes[RECEIPT] = receipt
    record = {'running': str(Path('/run/current-system').resolve(strict=True)),
              'boot': str(Path('/nix/var/nix/profiles/system').resolve(strict=True)),
              'previous_installer': os.readlink(ROOTS / 'installer') if (ROOTS / 'installer').is_symlink() else None,
              'activation_started': False, 'files': [], 'git_added': []}
    for path, after in changes.items():
        before = path.read_bytes() if path.exists() else None
        info = path.stat() if path.exists() else None
        record['files'].append({'path': str(path), 'before': base64.b64encode(before).decode() if before is not None else None,
                               'after_hash': digest(after), 'mode': stat.S_IMODE(info.st_mode) if info else (0o600 if path == RECEIPT else 0o644),
                               'uid': info.st_uid if info else 0, 'gid': info.st_gid if info else 0})
    roots(record)
    backup = STATE / 'backups' / (time.strftime('%Y%m%dT%H%M%S') + f'-{os.getpid()}.json')
    save_json(backup, record)
    save_json(PENDING, record)
    print(f'Private configuration backup: {backup}', flush=True)
    try:
        for item in record['files']:
            path = Path(item['path'])
            after = changes[path]
            if after is None:
                path.unlink(missing_ok=True)
            else:
                atomic(path, after, item['mode'], item['uid'], item['gid'])
        if flake and (Path('/etc/nixos/.git').exists()):
            for path in (MODULE, SELECTED):
                relative = str(path.relative_to('/etc/nixos'))
                if path.exists() and subprocess.run(['git', '-c', 'safe.directory=/etc/nixos', '-C', '/etc/nixos', 'ls-files', '--error-unmatch', relative],
                                                    stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
                    record['git_added'].append(relative)
                    save_json(PENDING, record)
                    run(['git', '-c', 'safe.directory=/etc/nixos', '-C', '/etc/nixos', 'add', '--intent-to-add', '--', relative])
        print('Building system integration; the Toolkit app and helper are already downloaded…', flush=True)
        with tempfile.TemporaryDirectory(prefix='toolkit-install-') as work:
            run(['nixos-rebuild', 'build', *args], cwd=work)
            target = Path(work, 'result').resolve(strict=True)
            if not str(target).startswith('/nix/store/'):
                raise RuntimeError('Rebuild returned an invalid system path.')
            if action == 'install':
                for name, key in [('nixos-toolkit', 'gui'), ('nixos-toolkit-helper', 'helper')]:
                    actual = target / 'sw/bin' / name
                    expected = Path(packages[key], 'bin', name)
                    if not actual.exists() or actual.resolve() != expected.resolve():
                        raise RuntimeError('The selected system did not include the new packages. Check the module imports and duplicate installations.')
            for item in record['files']:
                path = Path(item['path'])
                regular(path)
                current = path.read_bytes() if path.exists() else None
                if digest(current) != item['after_hash']:
                    raise RuntimeError('Configuration changed during installation; activation was cancelled.')
            if (str(Path('/run/current-system').resolve()) != record['running'] or
                    str(Path('/nix/var/nix/profiles/system').resolve()) != record['boot']):
                raise RuntimeError('Another operation changed the running or boot system; installation was cancelled.')
            record['activation_started'] = True
            save_json(PENDING, record)
            run(['nix-env', '--profile', '/nix/var/nix/profiles/system', '--set', target])
            run([target / 'bin/switch-to-configuration', 'switch'])
    except BaseException:
        print('Installation did not finish; restoring the previous configuration…', flush=True)
        restore(record)
        raise
    # Commit before nonessential cleanup/output. An output or cleanup failure
    # must not roll back a completed installation after discarding its journal.
    PENDING.unlink()
    directory = os.open(STATE, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)
    for name in ('previous-running', 'previous-boot', 'previous-installer'):
        (ROOTS / name).unlink(missing_ok=True)
    if action == 'uninstall':
        (ROOTS / 'installer').unlink(missing_ok=True)
    print('Uninstalled the app and helper; applied system settings were retained.' if action == 'uninstall'
          else 'Installed NixOS Toolkit and its helper. The app is available in your application menu.', flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['install', 'uninstall', 'recover'])
    parser.add_argument('--flake')
    args = parser.parse_args()
    if os.geteuid() != 0 or not Path('/etc/NIXOS').exists():
        parser.error('Run this signed installer through sudo on NixOS.')
    os.umask(0o077)
    STATE.mkdir(mode=0o700, parents=True, exist_ok=True)
    if STATE.is_symlink() or STATE.stat().st_uid != 0 or STATE.stat().st_mode & 0o077:
        raise RuntimeError(f'{STATE} must be a root-owned private directory.')
    with open('/run/lock/nixos-toolkit.lock', 'a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        if Path('/var/lib/nixos-toolkit/recovery').exists():
            raise RuntimeError('Finish Restore previous system in Toolkit before installing or uninstalling.')
        if args.action == 'recover':
            if PENDING.exists():
                restore(json.loads(PENDING.read_text()))
            else:
                print('No interrupted installation needs recovery.')
            return
        if PENDING.exists():
            raise RuntimeError('An interrupted installation needs recovery. Rerun this installer with --recover.')
        for sig in (signal.SIGTERM, signal.SIGINT, signal.SIGHUP):
            signal.signal(sig, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
        flake = args.flake
        if not flake and args.action == 'uninstall' and RECEIPT.exists():
            flake = json.loads(RECEIPT.read_text()).get('flake')
        transact(args.action, flake)


if __name__ == '__main__':
    try:
        main()
    except (RuntimeError, OSError, subprocess.CalledProcessError, KeyboardInterrupt) as error:
        print(f'Installer stopped: {error}', file=__import__('sys').stderr)
        raise SystemExit(1)
