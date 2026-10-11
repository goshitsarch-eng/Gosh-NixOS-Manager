# Run with a built helper path; all mutations happen in the disposable guest.
{ nixpkgs, helper }:
let
  pkgs = import nixpkgs { system = "x86_64-linux"; };
  helperPackage = builtins.storePath helper;
in
(import (nixpkgs + "/nixos/lib/testing-python.nix") { inherit pkgs; system = "x86_64-linux"; }).makeTest {
  name = "toolkit-reboot-recovery";
  nodes.machine = { lib, ... }: {
    virtualisation.memorySize = 768;
    virtualisation.diskSize = 2048;
    virtualisation.useBootLoader = true;
    boot.loader.grub.enable = true;
    environment.systemPackages = [ helperPackage pkgs.python3 ];
    environment.etc."toolkit-qa-marker".text = "baseline";
    specialisation.alternate.configuration.environment.etc."toolkit-qa-marker".text = lib.mkForce "alternate";
    system.autoUpgrade = { enable = true; dates = "2099-01-01"; };
    nix.gc = { automatic = true; dates = "2099-01-01"; };
  };
  testScript = ''
    import json, shlex
    machine.start()
    machine.wait_for_unit("multi-user.target")
    baseline = machine.succeed("readlink -f /run/current-system").strip()
    alternate = machine.succeed("readlink -f /run/current-system/specialisation/alternate").strip()
    profile = "/nix/var/nix/profiles/system"

    def request(kind, payload=None):
        req = {"type": kind}
        if payload is not None:
            req["payload"] = payload
        output = machine.succeed("printf '%s\\n' " + shlex.quote(json.dumps(req)) + " | nixos-toolkit-helper")
        replies = [json.loads(line) for line in output.splitlines()]
        return [reply for reply in replies if reply["type"] != "Log"][-1]

    def generation():
        return next(g["number"] for g in request("ListGenerations")["payload"] if g["current"])

    def reboot_to(marker):
        machine.shutdown()
        machine.start()
        machine.wait_for_unit("multi-user.target")
        assert machine.succeed("cat /etc/toolkit-qa-marker").strip() == marker

    machine.succeed("nix-env -p " + profile + " --set " + baseline)
    first = generation()
    machine.succeed("nix-env -p " + profile + " --set " + alternate)
    second = generation()
    assert request("RollbackGeneration", {"generation": second, "activate": "boot"})["type"] == "Ok"
    assert machine.succeed("cat /etc/toolkit-qa-marker").strip() == "baseline"
    reboot_to("alternate")
    assert request("RollbackGeneration", {"generation": first, "activate": "boot"})["type"] == "Ok"
    reboot_to("baseline")
    assert request("RollbackGeneration", {"generation": second, "activate": "switch"})["type"] == "Ok"
    assert machine.succeed("cat /etc/toolkit-qa-marker").strip() == "alternate"

    # Model a helper killed after activation. Isolated tests exercise the kill
    # and journal creation; this guest exercises actual system reactivation.
    record = {"mode": "Switch", "existed": True, "running": baseline,
              "selected": baseline, "started": True, "awaiting_state": False,
              "files_restored": False}
    machine.succeed("mkdir -p /var/lib/nixos-toolkit/recovery/managed/state /etc/nixos/nixos-toolkit/state")
    machine.succeed("chmod 700 /var/lib/nixos-toolkit /var/lib/nixos-toolkit/recovery")
    machine.succeed("printf original > /var/lib/nixos-toolkit/recovery/managed/state/qa-marker")
    machine.succeed("printf proposed > /etc/nixos/nixos-toolkit/state/qa-marker")
    machine.succeed("printf '%s' " + shlex.quote(json.dumps(record)) + " > /var/lib/nixos-toolkit/recovery/record.json")
    assert request("GetRecovery")["payload"] is not None
    assert request("RecoverApply")["type"] == "Ok"
    assert request("GetRecovery")["payload"] is None
    assert machine.succeed("cat /etc/nixos/nixos-toolkit/state/qa-marker").strip() == "original"
    assert machine.succeed("readlink -f /run/current-system").strip() == baseline
    assert machine.succeed("readlink -f " + profile).strip() == baseline
    reboot_to("baseline")
    machine.succeed("systemctl is-active nixos-upgrade.timer nix-gc.timer")
    machine.succeed("test -z \"$(systemctl --failed --no-legend)\"")
  '';
}
