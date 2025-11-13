use std::process::Command;
use std::env;

fn main() {
    // Get git commit hash (try git first, fall back to env var for Docker builds)
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| env::var("GIT_HASH").ok())
        .unwrap_or_else(|| "unknown".to_string());

    // Get git commit date (try git first, fall back to env var)
    let git_date = Command::new("git")
        .args(["log", "-1", "--format=%ci"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| env::var("GIT_DATE").ok())
        .unwrap_or_else(|| "unknown".to_string());

    // Get current branch (try git first, fall back to env var)
    let git_branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| env::var("GIT_BRANCH").ok())
        .unwrap_or_else(|| "unknown".to_string());

    // Check if working directory is clean (try git first, fall back to env var)
    let git_dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|output| !output.stdout.is_empty())
        .or_else(|| env::var("GIT_DIRTY").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(false);

    // Build timestamp
    let build_timestamp = chrono::Utc::now().to_rfc3339();

    // Rust version
    let rust_version = rustc_version::version().unwrap().to_string();

    // Pass to compiler
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=GIT_DATE={}", git_date);
    println!("cargo:rustc-env=GIT_BRANCH={}", git_branch);
    println!("cargo:rustc-env=GIT_DIRTY={}", git_dirty);
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", build_timestamp);
    println!("cargo:rustc-env=RUST_VERSION={}", rust_version);

    // Rebuild if git changes
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/index");
}
