fn main() {
    // StoreKit IAP (Mac App Store builds): compile swift/iap.swift into a static lib
    // and link it. Only when the `storekit` feature is on, so default/sandbox builds
    // never touch Swift/StoreKit. Enable with: pnpm tauri build --features storekit
    #[cfg(feature = "storekit")]
    build_storekit();

    tauri_build::build()
}

#[cfg(feature = "storekit")]
fn build_storekit() {
    use std::path::PathBuf;
    use std::process::Command;

    let src = "swift/iap.swift";
    println!("cargo:rerun-if-changed={src}");
    println!("cargo:rerun-if-env-changed=MACOSX_DEPLOYMENT_TARGET");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let lib = out.join("libmyappspot_iap.a");

    // Compile Swift for the SAME arch cargo is currently building, so the universal
    // (lipo) build links a matching slice. Without -target, swiftc defaults to the
    // host arch and the x86_64 pass fails with undefined StoreKit symbols.
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86_64") => "x86_64",
        other => panic!("unsupported target arch for StoreKit swift build: {other:?}"),
    };
    let deployment = std::env::var("MACOSX_DEPLOYMENT_TARGET").unwrap_or_else(|_| "13.0".into());
    let target = format!("{arch}-apple-macosx{deployment}");

    // swiftc -target <arch> -emit-library -static -parse-as-library -O -framework StoreKit -o <lib> <src>
    let status = Command::new("swiftc")
        .args([
            "-target",
            &target,
            "-emit-library",
            "-static",
            "-parse-as-library",
            "-O",
            "-framework",
            "StoreKit",
            "-o",
        ])
        .arg(&lib)
        .arg(src)
        .status()
        .expect("failed to run swiftc — is the Swift toolchain installed?");
    assert!(status.success(), "swiftc failed to build {src} for {target}");

    println!("cargo:rustc-link-search=native={}", out.display());
    println!("cargo:rustc-link-lib=static=myappspot_iap");
    println!("cargo:rustc-link-lib=framework=StoreKit");
}
