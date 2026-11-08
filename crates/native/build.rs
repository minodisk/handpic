use std::{env, process::Command};

fn main() {
    println!("cargo:rerun-if-env-changed=NASM");
    println!("cargo:rerun-if-env-changed=PATH");
    println!("cargo:rerun-if-env-changed=RIFFLE_NATIVE_ALLOW_NO_SIMD");

    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    if arch != "x86" && arch != "x86_64" {
        return;
    }
    let Err(problem) = check_nasm() else {
        return;
    };
    // mozjpeg-sys falls back to a build without SIMD with only a cargo:warning,
    // which cargo hides for a non-path dependency, so the slow decode would
    // otherwise pass unnoticed.
    if env::var_os("RIFFLE_NATIVE_ALLOW_NO_SIMD").is_some_and(|v| v == "1") {
        println!("cargo:warning={problem}; mozjpeg is built without SIMD");
        return;
    }
    eprintln!(
        "{problem}.\n\
         mozjpeg needs NASM for SIMD; without it JPEG decoding is about twice as slow.\n\
         Install it (`winget install NASM.NASM` on Windows, `apt-get install nasm` on \
         Debian / Ubuntu, `brew install nasm` on macOS) and put it on PATH,\n\
         or set RIFFLE_NATIVE_ALLOW_NO_SIMD=1 to build without SIMD."
    );
    std::process::exit(1);
}

fn check_nasm() -> Result<(), String> {
    let nasm = env::var_os("NASM").unwrap_or_else(|| "nasm".into());
    let shown = nasm.to_string_lossy().into_owned();
    let output = Command::new(&nasm)
        .arg("-v")
        .output()
        .map_err(|e| format!("`{shown} -v` did not run: {e}"))?;
    if !output.status.success() {
        return Err(format!("`{shown} -v` failed: {}", output.status));
    }
    let version = String::from_utf8_lossy(&output.stdout);
    if version.contains("NASM version 0.") {
        return Err(format!("NASM is too old: {}", version.trim()));
    }
    Ok(())
}
