#!/usr/bin/env python3
"""Exercise the real helper in an isolated Linux namespace, never on the host.

Requires bubblewrap, Python, and built helper/core binaries. The Nix store and
checkout are read-only; /etc/nixos, /run/lock, and Nix profiles are fixtures.
System commands are recording test doubles, so this does not test activation.
"""
import copy
import ctypes
import json
import os
from pathlib import Path
import shutil
import shlex
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def bootstrap():
    core = ctypes.CDLL(str(ROOT / "target/debug/libnixos_toolkit_core.so"))
    core.toolkit_call.argtypes = [ctypes.c_char_p, ctypes.c_size_t]
    core.toolkit_call.restype = ctypes.c_void_p
    core.toolkit_free.argtypes = [ctypes.c_void_p]
    request = b'{"operation":"bootstrap"}'
    pointer = core.toolkit_call(request, len(request))
    try:
        return json.loads(ctypes.string_at(pointer))["result"]
    finally:
        core.toolkit_free(pointer)


class HelperTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.catalog = bootstrap()

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="toolkit-helper-qa-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for name in ["etc/nixos", "run/lock", "profiles", "bin", "var", "gcroots"]:
            (self.root / name).mkdir(parents=True)
        (self.root / "etc/NIXOS").touch()
        (self.root / "etc/nixos/configuration.nix").write_text(
            "{ imports = [ ./nixos-toolkit/state/selected.nix ]; }"
        )
        self.state = copy.deepcopy(self.catalog["defaults"])
        self.state["hostname"] = "qa-desktop"
        self.managed = self.root / "etc/nixos/nixos-toolkit"
        mock = self.root / "bin/mock"
        mock.write_text(f"#!{sys.executable}\n" + '''
import json, os, pathlib, sys, time
name = pathlib.Path(sys.argv[0]).name
with open('/fixture/commands.jsonl', 'a') as out:
    out.write(json.dumps([name, *sys.argv[1:]]) + '\\n')
if name == 'nixos-rebuild':
    assert pathlib.Path('/etc/nixos/nixos-toolkit/state/selected.nix').exists()
    print('building fixture', flush=True)
    time.sleep(float(os.environ.get('QA_REBUILD_DELAY', '0')))
    code = os.environ.get('QA_PREFLIGHT_EXIT' if 'dry-build' in sys.argv else 'QA_REBUILD_EXIT', os.environ.get('QA_REBUILD_EXIT', '0'))
    sys.exit(int(code))
if name == 'nix-env' and '--list-generations' in sys.argv:
    print('1 2026-10-01 12:00:00\\n2 2026-10-02 12:00:00 (current)')
elif name == 'du':
    print('123M\\t/nix/store')
elif name == 'switch-to-configuration':
    print('activating fixture', flush=True)
    time.sleep(float(os.environ.get('QA_ACTIVATE_DELAY', '0')))
    sys.exit(int(os.environ.get('QA_ACTIVATE_EXIT', '0')))
''')
        mock.chmod(0o755)
        for name in ["nixos-rebuild", "nix-env", "nix-store", "nix-channel",
                     "nix", "nix-collect-garbage", "du"]:
            (self.root / "bin" / name).symlink_to("mock")
        activation = self.root / "profiles/system/bin"
        activation.mkdir(parents=True)
        (activation / "switch-to-configuration").symlink_to("/fixture/bin/mock")

    def command(self, **environment):
        command = [
            shutil.which("bwrap"), "--unshare-all", "--die-with-parent",
            "--ro-bind", "/nix/store", "/nix/store",
            "--ro-bind", str(ROOT), "/app",
            "--bind", str(self.root), "/fixture",
            "--bind", str(self.root / "etc"), "/etc",
            "--bind", str(self.root / "run"), "/run",
            "--bind", str(self.root / "profiles"), "/nix/var/nix/profiles",
            "--bind", str(self.root / "gcroots"), "/nix/var/nix/gcroots",
            "--bind", str(self.root / "var"), "/var",
            "--dev", "/dev", "--proc", "/proc", "--tmpfs", "/tmp",
            "--chdir", "/fixture", "--setenv", "PATH", "/fixture/bin",
            "--setenv", "TMPDIR", "/tmp",
            "--setenv", "NIXOS_TOOLKIT_TEMPLATES_DIR", "/app/nix/templates",
        ]
        for key, value in environment.items():
            command += ["--setenv", key, str(value)]
        return command + ["--", "/app/target/debug/nixos-toolkit-helper"]

    def run_requests(self, requests, **environment):
        result = subprocess.run(self.command(**environment),
                                input="".join(json.dumps(r) + "\n" for r in requests),
                                text=True, capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        replies = [json.loads(line) for line in result.stdout.splitlines()]
        return [reply for reply in replies if reply["type"] != "Log"]

    def apply_request(self, mode="Switch", state=None):
        payload = copy.deepcopy(self.state if state is None else state)
        payload.pop("last_applied", None)
        payload["rebuild_type"] = mode
        return {"type": "Apply", "payload": payload}

    def commands(self):
        file = self.root / "commands.jsonl"
        return [json.loads(line) for line in file.read_text().splitlines()] if file.exists() else []

    def seed(self):
        (self.managed / "state").mkdir(parents=True)
        (self.managed / "state/selected.nix").write_text("original config")
        (self.managed / "state/state.json").write_text(json.dumps(self.state))
        (self.managed / "notes.nix").write_text("local notes")

    def contents(self):
        return {str(p.relative_to(self.managed)): p.read_bytes()
                for p in self.managed.rglob("*") if p.is_file()}

    def test_configuration_revision_blocks_external_edits_before_any_writes(self):
        self.seed()
        snapshot = self.run_requests([{'type': 'ReadStateSnapshot'}])[0]
        self.assertEqual(snapshot['type'], 'StateSnapshot')
        self.assertEqual(snapshot['payload']['state']['hostname'], 'qa-desktop')
        revision = snapshot['payload']['revision']
        self.assertTrue(revision.startswith('v1:'))
        for changed in [self.root / 'etc/nixos/configuration.nix',
                        self.managed / 'state/selected.nix',
                        self.managed / 'state/state.json']:
            with self.subTest(changed=changed.name):
                original = changed.read_text()
                changed.write_text(original + '\n')
                before = self.contents()
                request = self.apply_request()
                request['payload']['expected_revision'] = revision
                response = self.run_requests([request])[0]
                self.assertEqual(response['type'], 'Error')
                self.assertIn('nothing was written', response['payload']['message'])
                self.assertEqual(self.contents(), before)
                self.assertEqual(self.commands(), [])
                self.assertFalse((self.root / 'var/lib/nixos-toolkit/recovery').exists())
                changed.write_text(original)
        request = self.apply_request('DryBuild')
        request['payload']['expected_revision'] = revision
        self.assertTrue(self.run_requests([request])[0]['payload']['success'])

    def test_configuration_snapshot_rejects_unreadable_nix_and_detects_new_files(self):
        self.seed()
        revision = self.run_requests([{'type': 'ReadStateSnapshot'}])[0]['payload']['revision']
        new = self.root / 'etc/nixos/manual-packages.nix'
        new.write_text('{ environment.systemPackages = []; }')
        request = self.apply_request('DryBuild')
        request['payload']['expected_revision'] = revision
        self.assertEqual(self.run_requests([request])[0]['type'], 'Error')
        new.unlink()
        new.symlink_to('/missing.nix')
        self.assertEqual(self.run_requests([{'type': 'ReadStateSnapshot'}])[0]['type'], 'Error')

    def test_all_rebuild_modes_and_saved_state(self):
        for mode in ["Switch", "Boot", "Test", "Build"]:
            with self.subTest(mode=mode):
                replies = self.run_requests([
                    self.apply_request(mode), {"type": "WriteState", "payload": {"state": self.state}},
                    {"type": "ReadState"},
                ])
                self.assertTrue(replies[0]["payload"]["success"])
                self.assertEqual(replies[1]["type"], "Ok")
                self.assertEqual(replies[2]["payload"], self.state)
                self.assertIn(["nixos-rebuild", mode.lower()], self.commands())
                self.assertEqual((self.managed / "state/selected.nix").stat().st_mode & 0o777, 0o644)
                self.assertEqual((self.managed / "state/state.json").stat().st_mode & 0o777, 0o600)

    def test_dry_build_restores_existing_tree_on_success_and_failure(self):
        self.seed()
        before = self.contents()
        for status in [0, 1]:
            reply = self.run_requests([self.apply_request("DryBuild")], QA_REBUILD_EXIT=status)[0]
            self.assertEqual(reply["payload"]["success"], status == 0)
            self.assertEqual(self.contents(), before)

    def test_failed_apply_restores_existing_tree(self):
        self.seed()
        before = self.contents()
        reply = self.run_requests([self.apply_request()], QA_REBUILD_EXIT=1)[0]
        self.assertFalse(reply["payload"]["success"])
        self.assertEqual(self.contents(), before)

    def test_first_apply_failure_and_dry_run_leave_no_managed_tree(self):
        for mode, status in [("Switch", 1), ("DryBuild", 0), ("DryBuild", 1)]:
            self.run_requests([self.apply_request(mode)], QA_REBUILD_EXIT=status)
            self.assertFalse(self.managed.exists())

    def test_write_failure_restores_existing_files(self):
        self.seed()
        (self.managed / "state/hostname.nix").mkdir()
        before = self.contents()
        reply = self.run_requests([self.apply_request()])[0]
        self.assertEqual(reply["type"], "Error")
        self.assertEqual(self.contents(), before)
        self.assertEqual(self.commands(), [])

    def test_flake_rebuild_uses_explicit_host_configuration(self):
        (self.root / "etc/nixos/flake.nix").write_text("{}")
        self.state['custom_packages'] = ['hello']
        self.state['selected_profile'] = 'xfce'
        self.state['enabled_bundles'] = ['devtools']
        self.run_requests([self.apply_request("Build")])
        self.assertIn(["nixos-rebuild", "build", "--flake", "/etc/nixos"], self.commands())
        selected = (self.managed / 'state/selected.nix').read_text()
        self.assertIn('hello', selected)
        self.assertNotIn('\n    ./custom-packages.nix\n', selected)
        self.assertNotIn('\n    ./hostname.nix\n', selected)
        reconstructed = self.run_requests([{'type': 'ReadState'}])[0]['payload']
        self.assertEqual(reconstructed['selected_profile'], 'xfce')
        self.assertEqual(reconstructed['enabled_bundles'], ['devtools'])
        self.assertEqual(reconstructed['custom_packages'], ['hello'])

    def test_apply_without_integration_does_not_report_false_success(self):
        (self.root / "etc/nixos/configuration.nix").write_text("{}")
        reply = self.run_requests([self.apply_request()])[0]
        self.assertEqual(reply["type"], "Error")
        self.assertIn("not imported", reply["payload"]["message"])
        self.assertFalse(self.managed.exists())
        self.assertEqual(self.commands(), [])

    def test_invalid_requests_do_not_write_or_run_commands(self):
        invalid = [
            ("hostname", "x\\n};"), ("selected_profile", "../escape"),
            ("user_groups", ["docker"]), ("custom_packages", ["a;abort"]),
            ("bundle_packages", {"gaming": ["not-in-catalog"]}),
        ]
        for key, value in invalid:
            state = copy.deepcopy(self.state)
            state[key] = value
            self.assertEqual(self.run_requests([self.apply_request(state=state)])[0]["type"], "Error", key)
        for key, value in [("ssh_port", 0), ("allowed_tcp_ports", [0]), ("ssh_root_login", "bad")]:
            state = copy.deepcopy(self.state)
            state["network_config"][key] = value
            self.assertEqual(self.run_requests([self.apply_request(state=state)])[0]["type"], "Error", key)
        state = copy.deepcopy(self.state)
        state["hardware_config"]["audio_server"] = 9
        self.assertEqual(self.run_requests([self.apply_request(state=state)])[0]["type"], "Error")
        self.assertFalse(self.managed.exists())
        self.assertEqual(self.commands(), [])

    def test_symlink_escape_cannot_touch_outside_files(self):
        target = self.root / "outside"
        target.mkdir()
        (target / "sentinel").write_text("unchanged")
        self.managed.symlink_to("/fixture/outside")
        replies = self.run_requests([self.apply_request(), {"type": "EnsureDirectories"}])
        self.assertTrue(all(r["type"] == "Error" for r in replies))
        self.assertEqual(list(target.iterdir()), [target / "sentinel"])

    def test_concurrent_mutating_session_is_rejected(self):
        first = subprocess.Popen(self.command(), stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                 stderr=subprocess.DEVNULL, text=True)
        try:
            first.stdin.write('{"type":"EnsureDirectories"}\n')
            first.stdin.flush()
            self.assertEqual(json.loads(first.stdout.readline())["type"], "Ok")
            second = self.run_requests([self.apply_request()])[0]
            self.assertEqual(second["type"], "Error")
            self.assertIn("another toolkit session", second["payload"]["message"])
        finally:
            first.communicate(timeout=10)

    def test_state_reconstruction_and_corrupt_json_recovery(self):
        self.run_requests([self.apply_request()])
        self.assertEqual(self.run_requests([{"type": "ReadState"}])[0]["payload"]["hostname"], "qa-desktop")
        (self.managed / "state/state.json").write_text("corrupt")
        self.assertEqual(self.run_requests([{"type": "ReadState"}])[0]["payload"]["hostname"], "qa-desktop")

    def test_reconstruction_preserves_bundle_aliases_without_adding_dependencies(self):
        selected = ["npm", "javascript", "flutter", "dart", "distroshelf"]
        self.state.update(enabled_bundles=["devtools"], bundle_packages={"devtools": selected})
        replies = self.run_requests([self.apply_request(mode="Build")])
        self.assertTrue(replies[0]["payload"]["success"])
        recovered = self.run_requests([{"type": "ReadState"}])[0]["payload"]
        self.assertEqual(recovered["bundle_packages"]["devtools"], selected)
        (self.managed / "state/state.json").write_text("corrupt")
        recovered = self.run_requests([{"type": "ReadState"}])[0]["payload"]
        self.assertEqual(recovered["bundle_packages"]["devtools"], selected)

    def test_interactive_cancel_and_disconnect_restore_files_and_release_lock(self):
        self.seed()
        before = self.contents()
        for disconnect in [False, True]:
            with self.subTest(disconnect=disconnect):
                process = subprocess.Popen(self.command(QA_REBUILD_DELAY=30), stdin=subprocess.PIPE,
                                           stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
                try:
                    process.stdin.write('{"type":"BeginSession"}\n')
                    process.stdin.flush()
                    self.assertEqual(json.loads(process.stdout.readline())["type"], "Ok")
                    process.stdin.write(json.dumps(self.apply_request()) + '\n')
                    process.stdin.flush()
                    while True:
                        response = json.loads(process.stdout.readline())
                        if response.get('payload', {}).get('message') == 'building fixture':
                            break
                    # Pipelined requests must not hide the running command's
                    # cancellation token. Both pending requests are canceled.
                    process.stdin.write(json.dumps(self.apply_request()) + '\n')
                    process.stdin.flush()
                    if not disconnect:
                        process.stdin.write('{"type":"Cancel"}\n')
                        process.stdin.flush()
                        while True:
                            response = json.loads(process.stdout.readline())
                            if response['type'] != 'Log':
                                self.assertEqual(response['type'], 'Error')
                                self.assertIn('interrupted', response['payload']['message'].lower())
                                break
                    process.stdin.close()
                    process.stdin = None
                    process.communicate(timeout=5)
                    self.assertEqual(process.returncode, 0)
                    self.assertEqual(self.contents(), before)
                    self.assertEqual(self.run_requests([{'type': 'EnsureDirectories'}])[0]['type'], 'Ok')
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.communicate(timeout=5)

    def test_generation_list_activation_and_deletion(self):
        replies = self.run_requests([
            {"type": "ListGenerations"},
            {"type": "RollbackGeneration", "payload": {"generation": 1, "activate": "boot"}},
            {"type": "DeleteGenerations", "payload": {"generations": [1]}},
            {"type": "DeleteGenerations", "payload": {"generations": [2]}},
            {"type": "DeleteGenerations", "payload": {"generations": []}},
        ])
        self.assertEqual([r["type"] for r in replies], ["Generations", "Ok", "Ok", "Error", "Error"])
        self.assertEqual(replies[0]["payload"][0]["number"], 2)
        self.assertIn(["switch-to-configuration", "boot"], self.commands())

    def test_preflight_failure_never_activates_and_preserves_selection(self):
        self.seed()
        before = self.contents()
        reply = self.run_requests([self.apply_request()], QA_PREFLIGHT_EXIT=1)[0]
        self.assertFalse(reply['payload']['success'])
        self.assertEqual(self.commands(), [['nixos-rebuild', 'dry-build']])
        self.assertEqual(self.contents(), before)
        self.assertIsNone(self.run_requests([{'type': 'GetRecovery'}])[0]['payload'])

    def test_integration_snippets_never_replace_existing_configuration(self):
        (self.managed / 'state').mkdir(parents=True)
        selected = self.managed / 'state/selected.nix'
        for name in ['classic_snippet', 'flake_snippet']:
            step = self.catalog[name].split('# Step 2:', 1)[1].split('# Step 3:', 1)[0]
            script = '\n'.join(step.splitlines()[1:]).replace('sudo sh', shlex.quote(shutil.which('sh')), 1)
            command = self.command()[:-1] + [shutil.which('sh'), '-c', script]
            if selected.exists():
                selected.unlink()
            subprocess.run(command, check=True, capture_output=True)
            self.assertIn('imports = []', selected.read_text())
            selected.write_text('keep the user selection exactly')
            subprocess.run(command, check=True, capture_output=True)
            self.assertEqual(selected.read_text(), 'keep the user selection exactly')

    def test_partial_activation_keeps_recovery_and_blocks_other_mutations(self):
        self.seed()
        before = self.contents()
        self.run_requests([self.apply_request()], QA_PREFLIGHT_EXIT=0, QA_REBUILD_EXIT=1)
        self.assertEqual(self.contents(), before)
        status = self.run_requests([{'type': 'GetRecovery'}])[0]['payload']
        self.assertTrue(status['files_restored'])
        self.assertIn('Services', status['message'])
        for request in [self.apply_request(), {'type': 'WriteState', 'payload': {'state': self.state}},
                        {'type': 'RunMaintenance', 'payload': {'command': 'nix-collect-garbage'}}]:
            self.assertIn('unfinished apply', self.run_requests([request])[0]['payload']['message'])
        # This fixture has no real previous system. Recovery must not claim
        # success or delete its backup when it cannot reactivate that system.
        self.assertEqual(self.run_requests([{'type': 'RecoverApply'}])[0]['type'], 'Error')
        self.assertIsNotNone(self.run_requests([{'type': 'GetRecovery'}])[0]['payload'])

    def test_unsaved_success_can_restore_original_files(self):
        self.seed()
        before = self.contents()
        original_mode = self.managed.stat().st_mode & 0o777
        self.run_requests([self.apply_request('Build')])
        status = self.run_requests([{'type': 'GetRecovery'}])[0]['payload']
        self.assertIn('saving the selection', status['message'])
        self.assertEqual(self.run_requests([{'type': 'RecoverApply'}])[0]['type'], 'Ok')
        self.assertEqual(self.contents(), before)
        self.assertEqual(self.managed.stat().st_mode & 0o777, original_mode)
        self.assertIsNone(self.run_requests([{'type': 'GetRecovery'}])[0]['payload'])

    def test_missing_recovery_copy_never_deletes_current_files(self):
        self.seed()
        self.run_requests([self.apply_request('Build')])
        current = self.contents()
        shutil.rmtree(self.root / 'var/lib/nixos-toolkit/recovery/managed')
        result = self.run_requests([{'type': 'RecoverApply'}])[0]
        self.assertEqual(result['type'], 'Error')
        self.assertIn('recovery copy is missing', result['payload']['details'])
        self.assertEqual(self.contents(), current)
        self.assertIsNotNone(self.run_requests([{'type': 'GetRecovery'}])[0]['payload'])

    def test_killed_helper_retains_durable_preflight_backup(self):
        self.seed()
        before = self.contents()
        process = subprocess.Popen(self.command(QA_REBUILD_DELAY=30), stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        try:
            process.stdin.write(json.dumps(self.apply_request()) + '\n')
            process.stdin.flush()
            while True:
                line = process.stdout.readline()
                self.assertTrue(line, 'Helper exited before preflight')
                if json.loads(line).get('payload', {}).get('message') == 'building fixture':
                    break
            process.kill()
            process.communicate(timeout=5)
        finally:
            if process.poll() is None:
                process.kill()
                process.communicate(timeout=5)
        status = self.run_requests([{'type': 'GetRecovery'}])[0]['payload']
        self.assertIsNotNone(status)
        backup = self.root / 'var/lib/nixos-toolkit/recovery'
        self.assertEqual(backup.stat().st_mode & 0o777, 0o700)
        self.assertEqual(self.run_requests([{'type': 'RecoverApply'}])[0]['type'], 'Ok')
        self.assertEqual(self.contents(), before)

    def test_corrupt_recovery_record_blocks_apply(self):
        journal = self.root / 'var/lib/nixos-toolkit/recovery'
        journal.mkdir(parents=True)
        (journal / 'record.json').write_text('corrupt')
        reply = self.run_requests([self.apply_request()])[0]
        self.assertEqual(reply['type'], 'Error')
        self.assertIn('recovery', reply['payload']['message'])
        self.assertEqual(self.commands(), [])

    def test_all_maintenance_commands_and_allowlist(self):
        for action in self.catalog["maintenance"]:
            reply = self.run_requests([{"type": "RunMaintenance", "payload": {"command": action["command"]}}])[0]
            self.assertEqual(reply["type"], "MaintenanceOutput")
            self.assertTrue(reply["payload"]["success"])
        self.assertNotIn(["nix-collect-garbage", "-d"], self.commands())
        self.assertIn(["nix-env", "-p", "/nix/var/nix/profiles/system", "--delete-generations", "1"], self.commands())
        before = self.commands()
        reply = self.run_requests([{"type": "RunMaintenance", "payload": {"command": "nix-store --optimise; reboot"}}])[0]
        self.assertEqual(reply["type"], "Error")
        self.assertEqual(self.commands(), before)

    def test_activation_failure_restores_previous_profile_selection(self):
        reply = self.run_requests([{"type": "RollbackGeneration", "payload": {"generation": 1, "activate": "switch"}}], QA_ACTIVATE_EXIT=1)[0]
        self.assertEqual(reply["type"], "Error")
        self.assertIn("System profile restored", reply["payload"]["details"])
        self.assertEqual(self.commands()[-1], ["nix-env", "-p", "/nix/var/nix/profiles/system", "--switch-generation", "2"])

    def test_cancelled_activation_still_restores_profile_selection(self):
        process = subprocess.Popen(self.command(QA_ACTIVATE_DELAY=30), stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        try:
            process.stdin.write('{"type":"BeginSession"}\n')
            process.stdin.flush()
            self.assertEqual(json.loads(process.stdout.readline())['type'], 'Ok')
            request = {'type': 'RollbackGeneration', 'payload': {'generation': 1, 'activate': 'switch'}}
            process.stdin.write(json.dumps(request) + '\n')
            process.stdin.flush()
            while True:
                response = json.loads(process.stdout.readline())
                if response.get('payload', {}).get('message') == 'activating fixture':
                    break
            process.stdin.write('{"type":"Cancel"}\n')
            process.stdin.flush()
            while True:
                response = json.loads(process.stdout.readline())
                if response['type'] != 'Log':
                    self.assertEqual(response['type'], 'Error')
                    self.assertIn('System profile restored', response['payload']['details'])
                    break
            process.stdin.close()
            process.stdin = None
            process.communicate(timeout=5)
            self.assertEqual(process.returncode, 0)
            self.assertEqual(self.commands()[-1], ['nix-env', '-p', '/nix/var/nix/profiles/system',
                                                 '--switch-generation', '2'])
        finally:
            if process.poll() is None:
                process.kill()
                process.communicate(timeout=5)

    def test_running_generation_is_protected_even_when_not_selected(self):
        (self.root / "profiles/generation-one").mkdir()
        (self.root / "profiles/system-1-link").symlink_to("generation-one")
        (self.root / "run/current-system").symlink_to("/nix/var/nix/profiles/system-1-link")
        replies = self.run_requests([
            {"type": "DeleteGenerations", "payload": {"generations": [1]}},
            {"type": "RunMaintenance", "payload": {"command": "nix-collect-garbage -d"}},
        ])
        self.assertEqual(replies[0]["type"], "Error")
        self.assertTrue(replies[1]["payload"]["success"])
        self.assertFalse(any("--delete-generations" in c for c in self.commands()))
        self.assertNotIn(["nix-collect-garbage", "-d"], self.commands())

    def test_disk_usage_and_host_detection(self):
        replies = self.run_requests([{"type": "GetDiskUsage"}, {"type": "GetSystemInfo"}, {"type": "CheckPermissions"}])
        self.assertEqual(replies[0]["payload"]["store_size"], "123M")
        self.assertEqual(replies[0]["payload"]["generation_count"], 2)
        self.assertTrue(replies[1]["payload"]["is_nixos"])
        self.assertTrue(replies[2]["payload"]["can_write_managed"])


if __name__ == "__main__":
    unittest.main(verbosity=2)
