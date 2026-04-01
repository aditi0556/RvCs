use crate::command::commit_tree::commit_tree;
use crate::command::refs::get_head_commit;
use crate::command::update_head::update_head;
use crate::command::write_tree::write_tree;
use crate::error::GitError;

pub fn commit(message: String) -> Result<(), GitError> {
    println!("Aditi");
    // 1. Create tree
    let tree_hash = write_tree(vec![])?;

    // 2. Get parent commit
    let parent = get_head_commit();

    // 3. Build args for commit_tree
    let mut args = vec![tree_hash.clone()];

    println!("Tree hash: {:?}", tree_hash);
    println!("Parent: {:?}", parent);
    println!("Args: {:?}", args);

    if let Some(parent_hash) = parent {
        if parent_hash.len() == 40 {
            args.push("-p".to_string());
            args.push(parent_hash);
        }
    }

    args.push("-m".to_string());
    args.push(message);

    // 4. Create commit object
    let commit_hash = commit_tree(args)?;
    println!("Commit_hash is {}", commit_hash);
    // 5. Update HEAD (branch ref)
    update_head(&commit_hash)?;

    println!("Committed: {}", commit_hash);

    Ok(())
}
