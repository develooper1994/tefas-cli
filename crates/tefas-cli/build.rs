fn main() {
    // Bake build-time info into the binary so --help all can display it at runtime.
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_PROFILE={profile}");
    println!("cargo:rustc-env=BUILD_TARGET={target}");

    // Collect active cargo features from CARGO_FEATURE_* environment variables.
    let mut features = Vec::new();
    if std::env::var("CARGO_FEATURE_NATIVE_TLS").is_ok() {
        features.push("native-tls");
    }
    if std::env::var("CARGO_FEATURE_HYPER_BACKEND").is_ok() {
        features.push("hyper-backend");
    }
    let features_str = if features.is_empty() {
        "none".to_string()
    } else {
        features.join(", ")
    };
    println!("cargo:rustc-env=BUILD_FEATURES={features_str}");
}
