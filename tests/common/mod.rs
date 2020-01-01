pub fn setup() {
    let temp_home = tempfile::Builder::new()
        .prefix("dotman-test-home")
        .tempdir()
        .expect("Couldn't create temporary home directory for testing");
}
