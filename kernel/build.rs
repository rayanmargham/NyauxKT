fn main() {
    if let Ok(_) = std::env::var("KERNEL") {
        println!("cargo:rustc-link-arg=-Tlinker.ld");
        // ..and to re-run if it changes.
        println!("cargo:rerun-if-changed=linker.ld");
        println!("cargo:rerun-if-changed=src/mem/gdt/flush.s");
    }
    // Tell cargo to pass the linker script to the linker..
}
