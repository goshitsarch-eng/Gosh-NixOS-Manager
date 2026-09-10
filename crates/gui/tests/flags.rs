//! Packaging-owned smoke flags: skip privileged init and host probes.

use nixos_toolkit_gui::Flags;

#[test]
fn for_tests_skips_privileged_init_and_host_probes() {
    let flags = Flags::for_tests();
    assert!(flags.skip_privileged_on_init);
    assert!(flags.skip_host_probes);
    assert!(!flags.spawn.helper_available);
    assert!(flags.spawn.program.as_os_str().is_empty());
}
