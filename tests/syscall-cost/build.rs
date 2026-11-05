fn main() {
    let script = std::env::var("DEP_EXECUTION_LINKER_SCRIPT").expect("execution linker script");
    println!("cargo::rustc-link-arg=-T{script}");
    println!("cargo::rerun-if-changed={script}");
}
