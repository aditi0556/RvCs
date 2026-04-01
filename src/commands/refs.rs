use std::fs;
use std::path::Path;

/// Update remote ref for a peer
pub fn update_remote_ref(peer_id: &str, hash: &str) {
    let dir = format!(".git/refs/remotes/");
    fs::create_dir_all(&dir).unwrap();
    let path = format!("{}/{}", dir, peer_id);
    fs::write(path, hash).unwrap();
}

/// Create a new branch
pub fn create_branch(branch_name: &str) -> std::io::Result<()> {
    let path = format!(".git/refs/heads/{}", branch_name);
    if Path::new(&path).exists() {
        println!("Branch '{}' already exists", branch_name);
        return Ok(());
    }
    // Initialize branch to point to current HEAD commit
    let head_commit = fs::read_to_string(".git/HEAD").unwrap_or_default();
    let mut commit_hash = head_commit.trim().to_string();
    if commit_hash.is_empty() {
        commit_hash = "0000000000000000000000000000000000000000".to_string(); // empty initial hash
    }
    fs::create_dir_all(".git/refs/heads")?;
    fs::write(&path, commit_hash)?;
    println!("Branch '{}' created at {}", branch_name, commit_hash);
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

    // Update HEAD to point to this branch
    let head_content = format!("refs/heads/{}", branch_name);
    fs::write(".git/HEAD", head_content)?;
    Ok(())
}

/// Get current branch
pub fn get_current_branch() -> Option<String> {
    let head_path = Path::new(".git/HEAD");
    if !head_path.exists() {
        return None;
    }

    let content = fs::read_to_string(head_path).ok()?;
    if content.starts_with("refs/heads/") {
        Some(content["refs/heads/".len()..].to_string())
    } else {
        None
    }
}