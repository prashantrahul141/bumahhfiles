use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output();

    let git_hash = match output {
        Ok(out) if out.status.success() => String::from_utf8(out.stdout)
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string(),
        _ => "unknown".to_string(),
    };

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    println!("cargo:rerun-if-changed=.git/HEAD");

    if let Ok(ref_bytes) = std::fs::read(".git/HEAD") {
        if let Some(ref_str) = String::from_utf8(ref_bytes).ok() {
            if ref_str.starts_with("ref:") {
                let ref_path = ref_str.trim_start_matches("ref: ").trim();
                println!("cargo::rerun-if-changed=.git/{}", ref_path);
            }
        }
    }
}
