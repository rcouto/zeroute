fn main() {
    println!("cargo:rustc-check-cfg=cfg(zerotier_core)");
    println!("cargo:rerun-if-env-changed=ZEROUTE_CORE_DIR");
    println!("cargo:rerun-if-changed=native/engine.cpp");
    println!("cargo:rerun-if-changed=native/engine.h");
    let Some(root) = std::env::var_os("ZEROUTE_CORE_DIR") else {
        return;
    };
    let root = std::path::PathBuf::from(root);
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .cpp_link_stdlib(None)
        .std("c++17")
        .include(root.join("include"))
        .file("native/engine.cpp")
        .compile("zeroute_engine_adapter");
    println!(
        "cargo:rustc-link-search=native={}",
        root.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=zerotiercore");
    for library in ["c++_static", "c++abi", "unwind"] {
        let output = build
            .get_compiler()
            .to_command()
            .arg(format!("--print-file-name=lib{library}.a"))
            .output()
            .expect("query Android compiler runtime");
        assert!(output.status.success(), "Android runtime query failed");
        let runtime = String::from_utf8(output.stdout).expect("runtime path is UTF-8");
        let runtime = std::path::Path::new(runtime.trim());
        assert!(
            runtime.is_absolute() && runtime.is_file(),
            "Android runtime missing: {library}"
        );
        // Do not add the NDK runtime directory to the global search path: it
        // also contains libc.a, which would shadow Android's shared libc stub.
        let output_dir = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
        std::fs::copy(runtime, output_dir.join(format!("lib{library}.a")))
            .expect("stage Android C++ runtime archive");
        println!("cargo:rustc-link-lib=static={library}");
    }
    println!("cargo:rustc-cfg=zerotier_core");
}
