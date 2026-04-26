// Must be the only test in this binary — set_dumpable taints the process state.
#[cfg(target_os = "linux")]
#[test]
fn set_dumpable_round_trips() {
    nix::sys::prctl::set_dumpable(false).unwrap();
    assert!(!nix::sys::prctl::get_dumpable().unwrap());
}
