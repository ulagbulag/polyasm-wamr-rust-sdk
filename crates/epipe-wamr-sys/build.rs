use std::{env, fs, path::Path, path::PathBuf, process::Command};

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

const ENABLED_DISABLES: &[&str] = &[
    "WAMR_DISABLE_APP_ENTRY",
    "WAMR_DISABLE_WAKEUP_BLOCKING_OP",
    "WAMR_DISABLE_WRITE_GS_BASE",
];

fn checked(command: &mut Command, phase: &str) {
    let status = command
        .status()
        .unwrap_or_else(|error| panic!("cannot execute WAMR {phase}: {error}"));
    assert!(status.success(), "WAMR {phase} failed with {status}");
}

fn source_root() -> PathBuf {
    if let Some(root) = env::var_os("WAMR_SOURCE_DIR") {
        return PathBuf::from(root);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../wamr-sys/wasm-micro-runtime")
}

fn configure(root: &Path, build: &Path) {
    let mut command = Command::new(env::var_os("CMAKE").unwrap_or_else(|| "cmake".into()));
    command
        .arg("-S")
        .arg(root)
        .arg("-B")
        .arg(build)
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
        command.arg(format!("-D{option}=0"));
    }
    for option in ENABLED_DISABLES {
        command.arg(format!("-D{option}=1"));
    }
    for (environment, option) in [
        ("WAMR_BUILD_PLATFORM", "WAMR_BUILD_PLATFORM"),
        ("WAMR_BUILD_TARGET", "WAMR_BUILD_TARGET"),
    ] {
        if let Ok(value) = env::var(environment) {
            command.arg(format!("-D{option}={value}"));
        }
    }
    checked(&mut command, "configuration");
}

fn main() {
    for variable in [
        "CARGO_CFG_TARGET_ARCH",
        "CARGO_CFG_TARGET_OS",
        "CMAKE",
        "CMAKE_BUILD_PARALLEL_LEVEL",
        "WAMR_BUILD_PLATFORM",
        "WAMR_BUILD_TARGET",
        "WAMR_SOURCE_DIR",
    ] {
        println!("cargo:rerun-if-env-changed={variable}");
    }

    let authoritative_root = source_root();
    assert!(
        authoritative_root.is_dir(),
        "missing pinned WAMR source: {}",
        authoritative_root.display()
    );
    for input in [
        authoritative_root.join("CMakeLists.txt"),
        authoritative_root.join("build-scripts"),
        authoritative_root.join("core/config.h"),
        authoritative_root.join("core/version.h"),
        authoritative_root.join("core/version.h.in"),
        authoritative_root.join("core/iwasm"),
        authoritative_root.join("core/shared"),
    ] {
        println!("cargo:rerun-if-changed={}", input.display());
    }

    let output = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let staged_root = output.join("wamr-source");
    let build = output.join("vmbuild");
    let cmake = env::var_os("CMAKE").unwrap_or_else(|| "cmake".into());
    let mut remove_stage = Command::new(&cmake);
    remove_stage
        .arg("-E")
        .arg("remove_directory")
        .arg(&staged_root);
    checked(&mut remove_stage, "source staging cleanup");
    let mut remove_build = Command::new(&cmake);
    remove_build.arg("-E").arg("remove_directory").arg(&build);
    checked(&mut remove_build, "build cleanup");
    for file in [
        "CMakeLists.txt",
        "core/config.h",
        "core/version.h",
        "core/version.h.in",
    ] {
        let source = authoritative_root.join(file);
        let destination = staged_root.join(file);
        fs::create_dir_all(destination.parent().unwrap())
            .unwrap_or_else(|error| panic!("cannot create WAMR source staging: {error}"));
        fs::copy(&source, &destination)
            .unwrap_or_else(|error| panic!("cannot stage {}: {error}", source.display()));
    }
    for directory in ["build-scripts", "core/iwasm", "core/shared"] {
        let mut copy_source = Command::new(&cmake);
        copy_source
            .arg("-E")
            .arg("copy_directory")
            .arg(authoritative_root.join(directory))
            .arg(staged_root.join(directory));
        checked(&mut copy_source, "source staging");
    }
    configure(&staged_root, &build);

    let mut compile = Command::new(cmake);
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
