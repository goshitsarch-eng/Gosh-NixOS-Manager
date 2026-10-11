#!/usr/bin/env python3
"""Test installer prompts/launchers with a private home and a recording Nix stub."""
import errno
import json
import os
from pathlib import Path
import pty
import select
import shlex
import signal
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]
EXPECTED = ["--extra-experimental-features", "nix-command flakes", "run",
            "github:goshitsarch-eng/Gosh-NixOS-Manager"]


class InstallerTests(unittest.TestCase):
    def run_installer(self, home, shell):
        pid, descriptor = pty.fork()
        if pid == 0:
            os.execvpe("bwrap", ["bwrap", "--unshare-all", "--die-with-parent",
                "--ro-bind", "/", "/", "--bind", str(home), os.environ["HOME"],
                "--tmpfs", "/tmp", "--ro-bind", str(ROOT), "/tmp/app", "--dev", "/dev",
                "--proc", "/proc", "--chdir", "/tmp/app", "bash", "/tmp/app/install.sh"],
                {**os.environ, "SHELL": shell,
                 "PATH": str(Path(os.environ["HOME"]) / "bin") + ":" + os.environ["PATH"]})
        output = b""
        prompts = [b"Create shell alias", b"Create desktop entry", b"Run NixOS Toolkit now?"]
        answered = 0
        deadline = time.monotonic() + 20
        try:
            while time.monotonic() < deadline:
                if select.select([descriptor], [], [], 0.1)[0]:
                    try:
                        chunk = os.read(descriptor, 65536)
                    except OSError as error:
                        if error.errno != errno.EIO:
                            raise
                        break
                    if not chunk:
                        break
                    output += chunk
                    if answered < len(prompts) and prompts[answered] in output:
                        os.write(descriptor, b"y")
                        answered += 1
            else:
                os.kill(pid, signal.SIGKILL)
                self.fail("Installer timed out: " + output.decode(errors="replace"))
        finally:
            os.close(descriptor)
            _, status = os.waitpid(pid, 0)
        self.assertEqual(os.waitstatus_to_exitcode(status), 0, output.decode(errors="replace"))
        self.assertEqual(answered, 3)

    def test_shell_launchers_desktop_arguments_and_repeat_install(self):
        import sys
        for shell, rc in [("/bin/bash", ".bashrc"), ("/bin/zsh", ".zshrc"),
                          ("/bin/fish", ".config/fish/config.fish"), ("", ".bashrc")]:
            with self.subTest(shell=shell), tempfile.TemporaryDirectory(prefix="toolkit-installer-qa-") as folder:
                home = Path(folder)
                (home / "bin").mkdir()
                stub = home / "bin/nix"
                stub.write_text(f"#!{sys.executable}\nimport json, os, sys\n"
                                "with open(os.path.join(os.environ['HOME'], 'nix-argv.json'), 'w') as f:\n"
                                "    json.dump(sys.argv[1:], f)\n")
                stub.chmod(0o755)
                self.run_installer(home, shell)
                self.assertEqual(json.loads((home / "nix-argv.json").read_text()), EXPECTED)
                config = (home / rc).read_text()
                self.run_installer(home, shell)
                self.assertEqual((home / rc).read_text(), config, "Alias must not be duplicated")
                desktop = (home / ".local/share/applications/nixos-toolkit.desktop").read_text()
                command = next(line.removeprefix("Exec=") for line in desktop.splitlines() if line.startswith("Exec="))
                self.assertEqual(shlex.split(command), ["nix", *EXPECTED])
                if shell.endswith("fish"):
                    self.assertIn("function nixos-toolkit;", config)
                    self.assertIn("'nix-command flakes'", config)
                else:
                    self.assertIn('"nix-command flakes"', config)


if __name__ == "__main__":
    unittest.main(verbosity=2)
