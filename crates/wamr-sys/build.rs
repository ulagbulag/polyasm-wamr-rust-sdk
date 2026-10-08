/*
 * Copyright (C) 2023 Liquid Reply GmbH. All rights reserved.
 * SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
 */

#[cfg(feature = "sdk-build")]
mod sdk {
    use cmake::Config;
    use std::{env, path::Path, path::PathBuf};

    const LLVM_LIBRARIES: &[&str] = &[
        "LLVMOrcJIT",
        "LLVMOrcShared",
        "LLVMOrcTargetProcess",
        "LLVMPasses",
        "LLVMProfileData",
        "LLVMRuntimeDyld",
        "LLVMScalarOpts",
        "LLVMSelectionDAG",
        "LLVMSymbolize",
        "LLVMTarget",
        "LLVMTextAPI",
        "LLVMTransformUtils",
        "LLVMVectorize",
        "LLVMX86AsmParser",
        "LLVMX86CodeGen",
        "LLVMX86Desc",
        "LLVMX86Disassembler",
        "LLVMX86Info",
        "LLVMXRay",
        "LLVMipo",
    ];

    fn check_is_espidf() -> bool {
        let is_espidf = env::var("CARGO_FEATURE_ESP_IDF").is_ok()
            && env::var("CARGO_CFG_TARGET_OS").unwrap() == "espidf";
        if is_espidf
            && (env::var("WAMR_BUILD_PLATFORM").is_ok()
                || env::var("WAMR_SHARED_PLATFORM_CONFIG").is_ok())
        {
            panic!("ESP-IDF build cannot use WAMR_BUILD_PLATFORM or WAMR_SHARED_PLATFORM_CONFIG");
        }
        is_espidf
    }

    fn feature_flags() -> (String, String, String, String, String, String) {
        (
            if cfg!(feature = "custom-section") {
                "1"
            } else {
                "0"
            }
            .into(),
            if cfg!(feature = "dump-call-stack") {
                "1"
            } else {
                "0"
            }
            .into(),
            if cfg!(feature = "llvmjit") { "1" } else { "0" }.into(),
            if cfg!(feature = "multi-module") {
                "1"
            } else {
                "0"
            }
            .into(),
            if cfg!(feature = "name-section") {
                "1"
            } else {
                "0"
            }
            .into(),
            if cfg!(feature = "hw-bound-check") {
                "0"
            } else {
                "1"
            }
            .into(),
        )
    }

    fn link_llvm_libraries(path: &str, enabled: &str) {
        if enabled == "0" {
            return;
        }
        let path = PathBuf::from(path);
        let lib = path.join("../../../lib").canonicalize().unwrap();
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=m");
        println!("cargo:rustc-link-lib=dylib=rt");
        println!("cargo:rustc-link-lib=dylib=stdc++");
        println!("cargo:rustc-link-lib=dylib=z");
        println!("cargo:rustc-link-search=native={}", lib.display());
        for library in LLVM_LIBRARIES {
            println!("cargo:rustc-link-lib=static={library}");
        }
    }

    fn config(root: &PathBuf) -> Config {
        let (custom, dump, llvm, multi, name, software_bounds) = feature_flags();
        let mut config = Config::new(root);
        config
            .define("WAMR_BUILD_AOT", "1")
            .define("WAMR_BUILD_INTERP", "1")
            .define("WAMR_BUILD_FAST_INTERP", "1")
            .define("WAMR_BUILD_JIT", &llvm)
            .define("WAMR_BUILD_BULK_MEMORY", "1")
            .define("WAMR_BUILD_REF_TYPES", "1")
            .define("WAMR_BUILD_SIMD", "1")
            .define("WAMR_BUILD_LIBC_WASI", "1")
            .define("WAMR_BUILD_LIBC_BUILTIN", "0")
            .define("WAMR_DISABLE_HW_BOUND_CHECK", &software_bounds)
            .define("WAMR_BUILD_MULTI_MODULE", &multi)
            .define("WAMR_BUILD_DUMP_CALL_STACK", &dump)
            .define("WAMR_BUILD_CUSTOM_NAME_SECTION", &name)
            .define("WAMR_BUILD_LOAD_CUSTOM_SECTION", &custom);
        if let Ok(value) = env::var("WAMR_BUILD_PLATFORM") {
            config.define("WAMR_BUILD_PLATFORM", value);
        }
        if let Ok(value) = env::var("WAMR_BUILD_TARGET") {
            config.define("WAMR_BUILD_TARGET", value);
        }
        if let Ok(value) = env::var("WAMR_SHARED_PLATFORM_CONFIG") {
            config.define("SHARED_PLATFORM_CONFIG", value);
        }
        if let Ok(value) = env::var("LLVM_LIB_CFG_PATH") {
            link_llvm_libraries(&value, &llvm);
            config.define("LLVM_DIR", value);
        }
        if let Ok(value) = env::var("WAMR_BH_VPRINTF") {
            config.define("WAMR_BH_VPRINTF", value);
        }
        config
    }

    fn build_runtime(root: &PathBuf) {
        let out = PathBuf::from(env::var("OUT_DIR").unwrap());
        let destination = config(root)
            .out_dir(out.join("vmbuild"))
            .build_target("vmlib")
            .build();
        println!(
            "cargo:rustc-link-search=native={}/build",
            destination.display()
        );
        println!("cargo:rustc-link-lib=static=iwasm");
    }

    fn build_compiler(root: &Path) {
        let out = PathBuf::from(env::var("OUT_DIR").unwrap());
        Config::new(root.join("wamr-compiler"))
            .out_dir(out.join("wamrcbuild"))
            .define("WAMR_BUILD_WITH_CUSTOM_LLVM", "1")
            .define(
                "LLVM_DIR",
                env::var("LLVM_LIB_CFG_PATH").expect("LLVM_LIB_CFG_PATH is required for sdk-build"),
            )
            .build();
    }

    fn generate_bindings(root: &Path) {
        let header = root.join("core/iwasm/include/wasm_export.h");
        let bindings = bindgen::Builder::default()
            .ctypes_prefix("::core::ffi")
            .use_core()
            .header(header.into_os_string().into_string().unwrap())
            .derive_default(true)
            .generate()
            .expect("unable to generate WAMR bindings");
        bindings
            .write_to_file(PathBuf::from(env::var("OUT_DIR").unwrap()).join("bindings.rs"))
            .expect("unable to write WAMR bindings");
    }

    pub fn run(root: &PathBuf) {
        if !check_is_espidf() {
            build_runtime(root);
            build_compiler(root);
        }
        generate_bindings(root);
    }
}

#[cfg(feature = "hermetic-interp")]
mod hermetic {
    use std::{env, path::Path, path::PathBuf, process::Command};

    const DISABLED_OPTIONS: &[&str] = &[
        "WAMR_BUILD_AOT",
        "WAMR_BUILD_BULK_MEMORY",
        "WAMR_BUILD_CUSTOM_NAME_SECTION",
        "WAMR_BUILD_DUMP_CALL_STACK",
        "WAMR_BUILD_EXCE_HANDLING",
        "WAMR_BUILD_FAST_INTERP",
        "WAMR_BUILD_FAST_JIT",
        "WAMR_BUILD_GC",
        "WAMR_BUILD_JIT",
        "WAMR_BUILD_LIBC_BUILTIN",
        "WAMR_BUILD_LIBC_WASI",
        "WAMR_BUILD_LIB_PTHREAD",
        "WAMR_BUILD_LIB_WASI_THREADS",
        "WAMR_BUILD_LINUX_PERF",
        "WAMR_BUILD_LOAD_CUSTOM_SECTION",
        "WAMR_BUILD_MEMORY64",
        "WAMR_BUILD_MEMORY_PROFILING",
        "WAMR_BUILD_MINI_LOADER",
        "WAMR_BUILD_MULTI_MEMORY",
        "WAMR_BUILD_MULTI_MODULE",
        "WAMR_BUILD_PERF_PROFILING",
        "WAMR_BUILD_REF_TYPES",
        "WAMR_BUILD_SHARED_MEMORY",
        "WAMR_BUILD_SIMD",
        "WAMR_BUILD_STRINGREF",
        "WAMR_BUILD_TAIL_CALL",
        "WAMR_BUILD_THREAD_MGR",
    ];

    fn checked(command: &mut Command, phase: &str) {
        let status = command
            .status()
            .unwrap_or_else(|error| panic!("cannot execute WAMR {phase}: {error}"));
        assert!(status.success(), "WAMR {phase} failed with {status}");
    }

    pub fn run(root: &Path) {
        let build = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("vmbuild");
        let mut configure = Command::new(env::var_os("CMAKE").unwrap_or_else(|| "cmake".into()));
        configure
            .arg("-S")
            .arg(root)
            .arg("-B")
            .arg(&build)
            .arg("-DBUILD_SHARED_LIBS=OFF")
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .arg("-DCMAKE_FIND_USE_PACKAGE_REGISTRY=FALSE")
            .arg("-DCMAKE_FIND_USE_SYSTEM_PACKAGE_REGISTRY=FALSE")
            .arg("-DFETCHCONTENT_FULLY_DISCONNECTED=ON")
            .arg("-DFETCHCONTENT_UPDATES_DISCONNECTED=ON")
            .arg("-DWAMR_BUILD_INTERP=1")
            .arg("-DWAMR_BUILD_SHRUNK_MEMORY=1")
            .arg("-DWAMR_DISABLE_HW_BOUND_CHECK=1");
        for option in DISABLED_OPTIONS {
            configure.arg(format!("-D{option}=0"));
        }
        for option in [
            "WAMR_DISABLE_APP_ENTRY",
            "WAMR_DISABLE_WAKEUP_BLOCKING_OP",
            "WAMR_DISABLE_WRITE_GS_BASE",
        ] {
            configure.arg(format!("-D{option}=1"));
        }
        for (environment, option) in [
            ("WAMR_BUILD_PLATFORM", "WAMR_BUILD_PLATFORM"),
            ("WAMR_BUILD_TARGET", "WAMR_BUILD_TARGET"),
        ] {
            if let Ok(value) = env::var(environment) {
                configure.arg(format!("-D{option}={value}"));
            }
        }
        checked(&mut configure, "configuration");

        let mut compile = Command::new(env::var_os("CMAKE").unwrap_or_else(|| "cmake".into()));
        compile
            .arg("--build")
            .arg(&build)
            .arg("--target")
            .arg("vmlib")
            .arg("--parallel");
        checked(&mut compile, "interpreter build");

        println!("cargo:rustc-link-search=native={}", build.display());
        println!("cargo:rustc-link-lib=static=iwasm");
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=dl");
    }
}

fn main() {
    let sdk = cfg!(feature = "sdk-build");
    let hermetic = cfg!(feature = "hermetic-interp");
    assert!(sdk ^ hermetic, "select exactly one WAMR build feature");

    for variable in [
        "CARGO_CFG_TARGET_ARCH",
        "CARGO_CFG_TARGET_OS",
        "CMAKE",
        "CMAKE_BUILD_PARALLEL_LEVEL",
        "LLVM_LIB_CFG_PATH",
        "WAMR_BH_VPRINTF",
        "WAMR_BUILD_PLATFORM",
        "WAMR_BUILD_TARGET",
        "WAMR_SHARED_PLATFORM_CONFIG",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }
    let root = env!("CARGO_MANIFEST_DIR");
    let root = std::path::PathBuf::from(root).join("wasm-micro-runtime");
    assert!(root.is_dir(), "missing pinned WAMR source");
    for input in [
        root.join("CMakeLists.txt"),
        root.join("build-scripts"),
        root.join("core/config.h"),
        root.join("core/version.h"),
        root.join("core/iwasm"),
        root.join("core/shared"),
    ] {
        println!("cargo:rerun-if-changed={}", input.display());
    }

    #[cfg(feature = "sdk-build")]
    sdk::run(&root);
    #[cfg(feature = "hermetic-interp")]
    hermetic::run(&root);
}
