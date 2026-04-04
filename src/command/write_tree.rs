use crate::error::GitError;
use crate::objects::GitObject;
use crate::command::add;
use std;
use std::collections::HashMap;
use std::path::Path;
use std::io::Write;

pub fn write_tree() -> Result<String, GitError> {
    let staged = add::get_staged_files();

    if staged.is_empty() {
        return Err(GitError::any("nothing to commit — stage files with git add"));
    }

    // Build a map: directory path → entries
    let mut dir_map: HashMap<String, Vec<(String, String, Vec<u8>)>> = HashMap::new();

    for (mode, blob_hash, filepath) in &staged {
        let hash_bytes = hex::decode(blob_hash)
            .map_err(|_| GitError::any(format!("invalid hash: {}", blob_hash)))?;

        let path = Path::new(filepath);

        let parent = path.parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();

        let name = path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| GitError::any(format!("invalid path: {}", filepath)))?
            .to_string();

        dir_map.entry(parent)
            .or_insert_with(Vec::new)
            .push((mode.clone(), name, hash_bytes));
    }

    // Build trees recursively starting from root
    build_tree_recursive("", &mut dir_map)
}

fn build_tree_recursive(dir: &str,dir_map: &mut HashMap<String, Vec<(String, String, Vec<u8>)>>,) -> Result<String, GitError> {
    let mut entries: Vec<(String, String, Vec<u8>)> = dir_map.remove(dir).unwrap_or_default();

    // Find immediate subdirectories
    let subdirs: Vec<String> = dir_map.keys()
        .filter(|k| {
            if dir.is_empty() {
                !k.is_empty() && !k.contains('/')
            } else {
                k.starts_with(&format!("{}/", dir)) &&
                !k[dir.len() + 1..].contains('/')
            }
        })
        .cloned()
        .collect();

    for subdir in subdirs {
        let subdir_name = if dir.is_empty() {
            subdir.clone()
        } else {
            subdir[dir.len() + 1..].to_string()
        };

        let subtree_hash = build_tree_recursive(&subdir, dir_map)?;
        let hash_bytes = hex::decode(&subtree_hash)
            .map_err(|_| GitError::any("invalid subtree hash"))?;

        entries.push(("40000".to_string(), subdir_name, hash_bytes));
    }

    // Sort entries lexicographically, dirs after files
    entries.sort_by(|a, b| {
        let a_name = if a.0 == "40000" { format!("{}/", a.1) } else { a.1.clone() };
        let b_name = if b.0 == "40000" { format!("{}/", b.1) } else { b.1.clone() };
        a_name.cmp(&b_name)
    });

    // Serialize tree content
    let mut contents = Vec::new();
    for (mode, name, hash_bytes) in &entries {
        contents.write_all(mode.as_bytes())?;
        contents.write_all(b" ")?;
        contents.write_all(name.as_bytes())?;
        contents.write_all(b"\0")?;
        contents.write_all(hash_bytes)?;
    }

    // Build and write tree object
    let tree_object = GitObject::build(crate::objects::Kind::Tree, contents)?;
    tree_object.write()?;
    Ok(tree_object.hex_string())
}


//this takes the snapshot of the directory and creates a tree hash
// pub fn write_tree(_args: Vec<String>) -> Result<String, GitError> {
//     let staged = add::get_staged_files();

//     if staged.is_empty() {
//         return Err(GitError::any("nothing to commit — stage files with git add"));
//     }
//     let git_object = match GitObject::from_path("./src", true) {
//         Ok(obj) => obj,
//         Err(e) => {
//             println!("ERROR in from_path: {:?}", e);
//             return Err(e);
//         }
//     };
//     let hash=git_object.hex_string();
//     println!("{}", git_object.hex_string());
//     Ok(hash)
// }

