fn main() {
    // This will search for 'x11' via pkg-config
    pkg_config::probe_library("x11").unwrap();
}
