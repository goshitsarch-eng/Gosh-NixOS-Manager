#!/usr/bin/env python3
"""Opt-in network activation regression test for a backed-up disposable VM."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cycles", type=int, default=6)
    parser.add_argument("--output", type=Path, default=Path("dist/qa-evidence/network-activation"))
    args = parser.parse_args()
    if (os.geteuid() != 0 or os.environ.get("NIXOS_TOOLKIT_LIVE_VM_QA") != "1"
            or not Path("/etc/NIXOS").exists()
            or not Path("/tmp/toolkit-vm-backup-location").exists()):
        parser.error("Requires root, NIXOS_TOOLKIT_LIVE_VM_QA=1 and a backed-up disposable NixOS VM")
    if not 1 <= args.cycles <= 20:
        parser.error("--cycles must be between 1 and 20")
    args.output.mkdir(parents=True, exist_ok=True)
    helper = "/run/current-system/sw/bin/nixos-toolkit-helper"
    report = []

    def request(kind, payload=None, name=None):
        req = {"type": kind}
        if payload is not None:
            req["payload"] = payload
        proc = subprocess.run([helper], input=json.dumps(req) + "\n", text=True,
                              capture_output=True, timeout=300, check=True)
        if name:
            (args.output / f"{name}.log").write_text(proc.stdout + proc.stderr)
        replies = [json.loads(line) for line in proc.stdout.splitlines()]
        return next(reply for reply in reversed(replies) if reply["type"] != "Log")

    def hashes():
        return {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                for p in Path("/etc/nixos/nixos-toolkit").rglob("*") if p.is_file()}

    def apply(state, name):
        draft = {k: v for k, v in state.items() if k != "last_applied"}
        reply = request("Apply", dict(draft, rebuild_type="Switch"), name)
        assert reply["type"] == "ApplyComplete" and reply["payload"]["success"], reply
        assert request("WriteState", {"state": state})["type"] == "Ok"
        assert request("GetRecovery")["payload"] is None

    assert request("GetRecovery")["payload"] is None, "Recover the host before starting"
    saved = request("ReadState")["payload"]
    environment = subprocess.check_output(
        ["systemctl", "show", "firewall", "-p", "Environment", "--value"], text=True)
    firewall_path = next((item.split("=", 1)[1] for item in shlex.split(environment)
                          if item.startswith("PATH=")), "")
    iptables = shutil.which("iptables", path=firewall_path)
    assert iptables, "This regression fixture requires the iptables firewall backend"

    def test_ports():
        rules = subprocess.check_output([iptables, "-w", "-S", "nixos-fw"], text=True)
        return {protocol: any(f"-p {protocol} " in rule and f"--dport {port} " in rule
                              for rule in rules.splitlines())
                for protocol, port in (("tcp", 18080), ("udp", 51821))}

    original_ports = test_ports()
    original_hashes = hashes()
    original_running = Path("/run/current-system").resolve()
    original_selected = Path("/nix/var/nix/profiles/system").resolve()
    # This suite expects its baseline to be active and selected for the next boot.
    assert original_running == original_selected
    try:
        for method in ("helper", "cli"):
            for cycle in range(1, args.cycles + 1):
                for enabled in (True, False):
                    state = copy.deepcopy(saved)
                    if enabled:
                        state["network_config"].update(
                            fail2ban_enabled=True, tailscale_enabled=True,
                            wireguard_enabled=True, allowed_tcp_ports=[18080],
                            allowed_udp_ports=[51821], wireguard_listen_port=51821)
                    name = f"{method}-{cycle}-{enabled}"
                    started = time.monotonic()
                    if method == "helper":
                        apply(state, name)
                    else:
                        draft = {k: v for k, v in state.items() if k != "last_applied"}
                        result = request("Generate", dict(draft, dry_run=False))
                        assert result["type"] == "GenerationResult", result
                        proc = subprocess.run(["nixos-rebuild", "switch"], text=True,
                                              capture_output=True, timeout=300)
                        (args.output / f"{name}.log").write_text(proc.stdout + proc.stderr)
                        assert proc.returncode == 0, f"{name}: inspect the activation log"
                        assert request("WriteState", {"state": state})["type"] == "Ok"
                    subprocess.run(["systemctl", "is-active", "firewall.service"],
                                   check=True, capture_output=True)
                    assert test_ports() == ({"tcp": True, "udp": True} if enabled else original_ports)
                    if enabled:
                        subprocess.run(["systemctl", "is-active", "fail2ban", "tailscaled"],
                                       check=True, capture_output=True)
                    assert not subprocess.check_output(
                        ["systemctl", "--failed", "--no-legend"], text=True).strip()
                    report.append({"method": method, "cycle": cycle, "enabled": enabled,
                                   "passed": True, "seconds": round(time.monotonic() - started, 2)})
                    (args.output / "report.json").write_text(json.dumps(report, indent=2))
                    print(f"PASS {name}", flush=True)
    finally:
        if request("GetRecovery")["payload"] is not None:
            assert request("RecoverApply", name="recovery")["type"] == "Ok"
        apply(saved, "restore-baseline")
        assert hashes() == original_hashes, "Managed files differ from the saved baseline"
        assert Path("/run/current-system").resolve() == original_running
        assert Path("/nix/var/nix/profiles/system").resolve() == original_selected
        assert test_ports() == original_ports
    print(f"PASS {len(report)} activations; managed files, running and boot systems restored")


if __name__ == "__main__":
    main()
