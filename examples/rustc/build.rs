fn main() {
    cc::Build::new()
        .cpp(true)
        .file("cpp/extra.cpp")
        .compile("nixwin_example");
}