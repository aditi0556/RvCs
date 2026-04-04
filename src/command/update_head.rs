use std::fs;
use std::path::Path;

pub fn update_head(commit_hash: &str) -> std::io::Result<()> {
    // First, figure out which branch HEAD points to
    if let Some(branch) = crate::command::refs::get_current_branch() {
        //here this format creates a string which is immediately dropped at the end of the statement
        //therefore dont pass it directly in the Path::new() otherwise the address to which it will point is already freed giving error
        let branch_path=format!(".git/refs/heads/{}", branch);
        let path = Path::new(&branch_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, commit_hash)?;
    } else {
        // Detached HEAD case: just overwrite HEAD with the commit hash
        fs::write(".git/HEAD", commit_hash)?;
    }

    Ok(())
}


pub fn update_remote_ref(peer_id: &str, hash: &str) {
    let dir = format!(".git/refs/remotes/");
    fs::create_dir_all(&dir).unwrap();

    let path = format!("{}/{}", dir, peer_id);

    fs::write(path, hash).unwrap();
}