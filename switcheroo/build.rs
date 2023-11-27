fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/arch/unix_x64.c");

    cc::Build::new()
        .file("src/arch/unix_x64.c")
        .compile("switcheroonaked");
}