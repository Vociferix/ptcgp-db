use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // Walk up from ptcgp-db/ to the workspace root where .git/ lives.
    let workspace_root = match manifest_dir.parent() {
        Some(p) => p.to_path_buf(),
        None => {
            println!("cargo:rustc-env=PTCGP_GIT_HASH=unknown");
            return;
        }
    };

    let git_head = workspace_root.join(".git/HEAD");
    println!("cargo:rerun-if-changed={}", git_head.display());

    // Also watch the current branch ref so every commit triggers a rebuild.
    if let Ok(head) = fs::read_to_string(&git_head)
        && let Some(refname) = head.trim().strip_prefix("ref: ")
    {
        let ref_path = workspace_root.join(".git").join(refname);
        println!("cargo:rerun-if-changed={}", ref_path.display());
    }

    let hash = read_git_hash(&workspace_root).unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=PTCGP_GIT_HASH={hash}");
}

fn read_git_hash(workspace: &Path) -> Option<String> {
    let head = fs::read_to_string(workspace.join(".git/HEAD")).ok()?;
    let head = head.trim();

    let full_hash = if let Some(refname) = head.strip_prefix("ref: ") {
        let ref_path = workspace.join(".git").join(refname);
        match fs::read_to_string(&ref_path) {
            Ok(h) => h.trim().to_string(),
            Err(_) => resolve_packed_ref(workspace, refname)?,
        }
    } else {
        // Detached HEAD: the content is the commit hash itself.
        head.to_string()
    };

    full_hash.get(..7).map(str::to_string)
}

fn resolve_packed_ref(workspace: &Path, refname: &str) -> Option<String> {
    let packed = fs::read_to_string(workspace.join(".git/packed-refs")).ok()?;
    packed
        .lines()
        .filter(|l| !l.starts_with('#') && !l.starts_with('^'))
        .find(|l| l.ends_with(refname))?
        .split_whitespace()
        .next()
        .map(str::to_string)
}
