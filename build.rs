use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Tell Cargo to rerun this script if the man page changes
    println!("cargo:rerun-if-changed=docs/eka.1");

    // Get the OUT_DIR where we should put our files
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = PathBuf::from(&out_dir);

    // Copy the man page to the output directory
    fs::copy("docs/eka.1", dest_path.join("eka.1")).unwrap();

    // If building for installation, also print the man page path
    if env::var("CARGO_FEATURE_MANPAGE").is_ok() {
        println!("cargo:rustc-env=MANPAGE_PATH={}/eka.1", out_dir);
    }
}
