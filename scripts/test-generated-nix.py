#!/usr/bin/env python3
"""Generate and evaluate every selectable module without building/activating it.

Usage: python3 scripts/test-generated-nix.py /absolute/path/to/nixpkgs [filter|--failed]
Reports and generated modules are retained under dist/qa-generated-nix/.
"""
import copy
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("helper_qa", ROOT / "scripts/test-helper-isolated.py")
helper_qa = importlib.util.module_from_spec(spec)
spec.loader.exec_module(helper_qa)


def cases(catalog):
    def state(**values):
        result = copy.deepcopy(catalog["defaults"])
        result.update(values)
        return result
    yield "empty", state()
    yield "identity", state(hostname="qa-desktop", dns_servers=["1.1.1.1"], username="alice", user_groups=["docker", "libvirtd", "vboxusers"], custom_packages=["ripgrep", "fd"])
    for profile in catalog["profiles"]:
        yield "profile-" + profile["id"], state(selected_profile=profile["id"])
        yield "fallback-profile-" + profile["id"], state(selected_profile=profile["id"])
    for bundle in catalog["bundles"]:
        name = bundle["id"]
        yield "bundle-template-" + name, state(enabled_bundles=[name])
        yield "bundle-selected-" + name, state(enabled_bundles=[name], bundle_packages={name: [p["id"] for p in bundle["packages"] if not p.get("unavailable_reason")]})
        yield "bundle-empty-" + name, state(enabled_bundles=[name], bundle_packages={name: []})
        if name in {"ai-tools", "devtools", "flatpak"}:
            for package in bundle["packages"]:
                if not package.get("unavailable_reason"):
                    yield f"bundle-package-{name}-{package['id']}", state(enabled_bundles=[name], bundle_packages={name: [package["id"]]})
    for service in catalog["defaults"]["services_config"]:
        value = state()
        value["services_config"][service] = True
        yield "service-" + service, value
    value = state()
    value["services_config"] = dict.fromkeys(value["services_config"], True)
    yield "services-all", value
    for key, values in {
        "nvidia_driver": [0, 1, 2, 3], "audio_server": [0, 1, 2],
        "power_profile": [0, 1, 2], "audio_lowlatency": [True],
        "bluetooth_enabled": [True], "tlp_enabled": [True], "thermald_enabled": [True],
    }.items():
        for option in values:
            value = state()
            value["hardware_config"][key] = option
            yield f"hardware-{key}-{option}", value
    for key, option in {
        "firewall_enabled": False, "ssh_enabled": True, "fail2ban_enabled": True,
        "tailscale_enabled": True, "wireguard_enabled": True,
        "allowed_tcp_ports": [22, 443], "allowed_udp_ports": [53, 51820],
    }.items():
        value = state()
        value["network_config"][key] = option
        yield "network-" + key, value
    value = state()
    value["network_config"].update(ssh_enabled=True, fail2ban_enabled=True, ssh_port=2222,
                                   tailscale_enabled=True, wireguard_enabled=True, wireguard_listen_port=51821)
    yield "network-all", value
    for profile in ["gnome", "kde"]:
        for key, option in [("tlp_enabled", True), ("audio_server", 1), ("audio_server", 2)]:
            value = state(selected_profile=profile)
            value["hardware_config"][key] = option
            yield f"combined-{profile}-{key}-{option}", value
    value = state(enabled_bundles=["devtools", "containers"])
    yield "combined-docker-podman", value


def main():
    nixpkgs = Path(sys.argv[1]).resolve()
    selected = sys.argv[2] if len(sys.argv) > 2 else ""
    destination = Path(os.environ.get("TOOLKIT_QA_DIR", ROOT / "dist/qa-generated-nix"))
    destination.mkdir(parents=True, exist_ok=True)
    report = json.loads((destination / "report.json").read_text()) if selected == "--failed" else []
    failed = {entry["case"] for entry in report if not entry["passed"]}
    for name, state in cases(helper_qa.bootstrap()):
        if selected == "--failed" and name not in failed:
            continue
        if selected != "--failed" and selected not in name:
            continue
        state.pop("last_applied", None)
        request = {"type": "Generate", "payload": {**state, "dry_run": True}}
        result = subprocess.run([str(ROOT / "target/debug/nixos-toolkit-helper")],
                                input=json.dumps(request) + "\n", text=True, capture_output=True,
                                env={**os.environ, "NIXOS_TOOLKIT_TEMPLATES_DIR": "/nonexistent-qa-templates" if name.startswith("fallback-profile-") else str(ROOT / "nix/templates")},
                                timeout=30, check=True)
        reply = json.loads(result.stdout.splitlines()[-1])
        if reply["type"] != "GenerationResult":
            raise RuntimeError(reply)
        folder = destination / name
        for file in reply["payload"]["files"]:
            path = folder / Path(file["path"]).relative_to("/etc/nixos/nixos-toolkit")
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(file["content"])
        expression = f'''let system = import {nixpkgs}/nixos {{
          configuration = {{ lib, ... }}: {{
            imports = [ {folder}/state/selected.nix ];
            nixpkgs.hostPlatform = "x86_64-linux";
            system.stateVersion = "25.05";
            boot.isContainer = true;
            networking.useHostResolvConf = false;
            networking.hostName = lib.mkOverride 1100 "qa";
            users.users.alice.isNormalUser = true;
          }};
        }}; in system.config.system.build.toplevel.drvPath'''
        checked = subprocess.run(["nix-instantiate", "--eval", "--strict", "--json", "--expr", expression],
                                 text=True, capture_output=True, timeout=180)
        (folder / "evaluation.log").write_text(checked.stderr)
        passed = checked.returncode == 0
        report = [entry for entry in report if entry["case"] != name]
        report.append({"case": name, "passed": passed})
        print(f"{'PASS' if passed else 'FAIL'} {name}", flush=True)
        if not passed:
            print(checked.stderr[-1800:], flush=True)
        (destination / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(f"{sum(r['passed'] for r in report)}/{len(report)} module evaluations passed")
    return 0 if all(r["passed"] for r in report) else 1


if __name__ == "__main__":
    sys.exit(main())
