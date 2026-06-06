use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    // Walk up from ptcgp-db/ to the workspace root where .git/ lives.
    let workspace_root = match manifest_dir.parent() {
        Some(p) => p.to_path_buf(),
        None => {
            emit_fallback();
            return;
        }
    };

    let git_head = workspace_root.join(".git/HEAD");
    println!("cargo:rerun-if-changed={}", git_head.display());
    // Also watch the current branch ref so every commit triggers a rebuild.
    if let Ok(head) = std::fs::read_to_string(&git_head)
        && let Some(refname) = head.trim().strip_prefix("ref: ")
    {
        let ref_path = workspace_root.join(".git").join(refname);
        println!("cargo:rerun-if-changed={}", ref_path.display());
    }

    match git_version_info() {
        Ok((version, hash)) => {
            println!("cargo:rustc-env=PTCGP_APP_VERSION={version}");
            println!("cargo:rustc-env=PTCGP_GIT_HASH={hash}");
        }
        Err(e) => {
            println!("cargo:warning=failed to read git info: {e}");
            emit_fallback();
        }
    }
}

/// Falls back to the Cargo.toml version when git is unavailable (e.g. source-only distributions).
fn emit_fallback() {
    let version = env::var("CARGO_PKG_VERSION")
        .map(|v| format!("v{v}"))
        .unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=PTCGP_APP_VERSION={version}");
    println!("cargo:rustc-env=PTCGP_GIT_HASH=unknown");
}

fn git_version_info() -> Result<(String, String), git2::Error> {
    let repo = git2::Repository::discover(".")?;

    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let full_hash = commit.id().to_string();
    let short_hash = full_hash.get(..7).unwrap_or(&full_hash).to_string();

    let version = describe_version(&repo, &short_hash);

    Ok((version, short_hash))
}

/// Returns `v1.2.3` when HEAD is exactly on a version tag, or `v1.2.3-5-gabcdef (dev)`
/// when there are additional commits. Falls back to `{hash} (dev)` if no matching tag
/// is reachable (e.g. shallow clone without tag history).
fn describe_version(repo: &git2::Repository, short_hash: &str) -> String {
    let Ok(desc) = repo.describe(
        git2::DescribeOptions::new()
            .describe_tags()
            .pattern("v[0-9]*"),
    ) else {
        return format!("{short_hash} (dev)");
    };

    let Ok(s) = desc.format(Some(git2::DescribeFormatOptions::new().abbreviated_size(7))) else {
        return format!("{short_hash} (dev)");
    };

    // Tags are "vX.Y.Z" with no hyphens, so any "-" in the output indicates
    // additional commits since the last tag. Strip "-N-gabcdef" so only the
    // tag version is shown; the commit hash is displayed separately in the UI.
    if let Some((tag, _)) = s.split_once('-') {
        format!("{tag} (dev)")
    } else {
        s
    }
}
