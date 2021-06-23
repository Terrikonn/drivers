fn main() {
    #[cfg(not(target_arch = "x86_64"))]
    compile_error!("Expected x86_64 arch");
}
