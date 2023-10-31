//! Pokedex build script.

use rustc_version::version_meta;
use rustc_version::Channel::Nightly;

#[doc(hidden)]
fn main() {
    // If migrations change on disk, we want to rebuild the `run_migrations` binary.
    println!("cargo:rerun-if-changed=migrations");

    // Backtrace exists in stable, but to use it with std::error::Error,
    // we need to be on the Nightly channel at least.
    if version_meta().unwrap().channel <= Nightly {
        println!("cargo:rustc-cfg=backtrace_support");
    }
}
