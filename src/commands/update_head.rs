use std::fs;
use std::path::Path;

pub fn update_head(commit_hash: &str) -> std::io::Result<()> {
    let path = Path::new(".git/refs/heads/main");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, commit_hash)?;
    Ok(())
}

pub fn update_remote_ref(peer_id: &str, hash: &str) {
    let dir = format!(".git/refs/remotes/");
    fs::create_dir_all(&dir).unwrap();

    let path = format!("{}/{}", dir, peer_id);

    fs::write(path, hash).unwrap();
}