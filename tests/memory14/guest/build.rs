fn main() {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo:rustc-link-arg-bin=memory14-guest=-T{root}/../../memory/guest/link.ld");
    println!("cargo:rerun-if-changed=../../memory/guest/link.ld");
}
