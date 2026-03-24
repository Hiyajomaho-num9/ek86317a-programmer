use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    tauri_build::build();

    if env::var_os("CARGO_FEATURE_CH347F").is_some() {
        configure_ch347();
    }
}

fn configure_ch347() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set"));
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();

    match target_os.as_str() {
        "linux" => configure_linux(&manifest_dir, &target_arch),
        "windows" => configure_windows(&manifest_dir, &target_arch),
        _ => {
            println!("cargo:warning=CH347F backend is only wired for Windows and Linux targets");
        }
    }
}

fn configure_linux(manifest_dir: &Path, target_arch: &str) {
    let arch_dir = match target_arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        other => panic!(
            "CH347F Linux backend does not provide a vendored library for target arch {other}"
        ),
    };

    let lib_dir = manifest_dir.join("vendor/ch347/linux").join(arch_dir);
    let lib_path = lib_dir.join("libch347.a");
    if !lib_path.exists() {
        panic!(
            "Missing vendored CH347 Linux static library at {}",
            lib_path.display()
        );
    }

    println!("cargo:rerun-if-changed={}", lib_path.display());
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=static=ch347");
}

fn configure_windows(manifest_dir: &Path, target_arch: &str) {
    let dll_path = match target_arch {
        "x86_64" => manifest_dir.join("vendor/ch347/windows/amd64/CH347DLLA64.dll"),
        other => {
            panic!("CH347F Windows backend does not provide a vendored DLL for target arch {other}")
        }
    };

    if !dll_path.exists() {
        panic!(
            "Missing vendored CH347 Windows DLL at {}",
            dll_path.display()
        );
    }

    println!("cargo:rerun-if-changed={}", dll_path.display());

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR is set"));
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .expect("OUT_DIR should be nested under target/<profile>/build")
        .to_path_buf();

    copy_file(&dll_path, &profile_dir.join("CH347DLLA64.dll"));
    copy_file(&dll_path, &profile_dir.join("deps").join("CH347DLLA64.dll"));
}

fn copy_file(src: &Path, dst: &Path) {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).expect("failed to create destination directory");
    }
    fs::copy(src, dst).unwrap_or_else(|err| {
        panic!(
            "failed to copy CH347 runtime from {} to {}: {}",
            src.display(),
            dst.display(),
            err
        )
    });
}
