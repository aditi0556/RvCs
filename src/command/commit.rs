use crate::command::refs::get_head_commit;
use crate::command::update_head::update_head;
use crate::command::write_tree::write_tree;
use crate::error::GitError;
use crate::command::add;
use crate::command::refs;
use crate::objects;
pub fn commit(message: String) -> Result<(), GitError> {

    // 1. check if a merge is in progress
    let merge_head = std::fs::read_to_string(".git/MERGE_HEAD")
        .ok() 
        .map(|s| s.trim().to_string())
        .filter(|s| s.len() == 40);

    // 2. check staged files exists
    let staged = add::get_staged_files();
    if staged.is_empty() {
        return Err(GitError::any(
            "nothing to commit — stage files with: git add <file>",
        ));
    }

    // 3. if merge in progress, check ALL staged files for conflict markers 
    // just like git refuses to commit if any file still has <<<<<
    
    if merge_head.is_some() {
        let mut unresolved: Vec<String> = Vec::new();

        for (_, _, filepath) in &staged {
            let content = std::fs::read_to_string(filepath)
                .unwrap_or_default();

            // check for any of the three conflict marker types
            let has_conflict = content.lines().any(|line| {
                line.starts_with("<<<<<<< LOCAL")
                    || line.starts_with("=======")
                    || line.starts_with(">>>>>>> REMOTE")
            });

            if has_conflict {
                unresolved.push(filepath.clone());
            }
        }

        if !unresolved.is_empty() {
            println!("error: Committing is not possible because you have unresolved files.");
            println!("hint:  Fix all conflicts and then mark them as resolved with git add <file>");
            println!();
            for f in &unresolved {
                println!("  conflict:  {}", f);
            }
            println!();
            println!("You are in the middle of a merge — resolve conflicts then commit.");
            return Err(GitError::any("unresolved conflicts"));
        }
    }
    //  4. build tree from index
    let tree_hash = write_tree()?; 
    // 5. get local HEAD as first parent
    let local_parent = get_head_commit();
    // 6. build commit object
    let commit_hash = match &merge_head {
        None => {
            // normal commit — use existing build_commit (single parent)
            let obj = objects::GitObject::build_commit(
                &message,
                &tree_hash,
                local_parent.as_deref(),
            )?;
            obj.write()?;
            obj.hex_string()
        }
        Some(mh) => {
            // merge commit — two parents, build manually
            // because build_commit only handles one parent
            let mut contents = String::new();
            use std::fmt::Write;

            let committer = "Code Crafters <000000000+codecrafters@users.noreply.github.com> 1750973235 +0000";

            writeln!(contents, "tree {}", tree_hash)?;

            // first parent = our local HEAD
            if let Some(ref p) = local_parent {
                writeln!(contents, "parent {}", p)?;
            }
            // second parent = the remote HEAD we merged from (MERGE_HEAD)
            writeln!(contents, "parent {}", mh)?;

            writeln!(contents, "author {}", committer)?;
            writeln!(contents, "committer {}", committer)?;
            writeln!(contents)?;
            writeln!(contents, "{}", message)?;

            let obj = objects::GitObject::build(objects::Kind::Commit, contents.into_bytes())?;
            obj.write()?;
            obj.hex_string()
        }
    };
    // 7. update HEAD ref
    update_head(&commit_hash)?;
    //  8. clear index 
    add::clear_index()?;

    //  9. clean up merge state 
    if merge_head.is_some() {
        std::fs::remove_file(".git/MERGE_HEAD").ok();
        std::fs::remove_file(".git/MERGE_MSG").ok();
        println!("Merge commit: {}", &commit_hash[..7]);
        println!("  branch '{:?}' merged successfully.", refs::get_current_branch());
    } else {
        println!("Committed: {}", &commit_hash[..7]);
    }

    Ok(())
}
