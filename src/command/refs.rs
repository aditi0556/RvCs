use std::fs;
use std::path::Path;

/// Resolve HEAD → actual commit hash
pub fn get_head_commit() -> Option<String> {
    let head = fs::read_to_string(".git/HEAD").ok()?;

    // symbolic ref
    if let Some(ref_path) = head.strip_prefix("ref: ") {
        let ref_path = ref_path.trim();
        let commit = fs::read_to_string(format!(".git/{}", ref_path)).ok()?;
        return Some(commit.trim().to_string());
    }

    // detached HEAD
    Some(head.trim().to_string())
}

/// Update remote ref for a peer
// pub fn update_remote_ref(peer_id: &str, hash: &str) {
//     let dir = ".git/refs/remotes/";
//     fs::create_dir_all(dir).unwrap();
//     let path = format!("{}/{}", dir, peer_id);
//     fs::write(path, hash).unwrap();
// }

/// Create a new branch
pub fn create_branch(branch_name: &str) -> std::io::Result<()> {
    let path = format!(".git/refs/heads/{}", branch_name);

    if Path::new(&path).exists() {
        println!("Branch '{}' already exists", branch_name);
        return Ok(());
    }

    // resolve HEAD properly
    let commit_hash = get_head_commit()
        .unwrap_or_else(|| "0000000000000000000000000000000000000000".to_string());

    fs::create_dir_all(".git/refs/heads")?;
    fs::write(&path, commit_hash)?;
    Ok(())
}

/// Switch to a branch
pub fn switch_branch(branch_name: &str) -> std::io::Result<()> {
    let branch_path = format!(".git/refs/heads/{}", branch_name);

    if !Path::new(&branch_path).exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Branch '{}' does not exist", branch_name),
        ));
    }

    //correct HEAD format
    let head_content = format!("ref: refs/heads/{}", branch_name);
    fs::write(".git/HEAD", head_content)?;

    Ok(())
}

// Get current branch
pub fn get_current_branch() -> Option<String> {
    let content = fs::read_to_string(".git/HEAD").ok()?;

    // handle "ref: "
    if let Some(branch) = content.strip_prefix("ref: refs/heads/") {
        Some(branch.trim().to_string())
    } else {
        None
    }
}