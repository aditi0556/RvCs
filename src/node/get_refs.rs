use std::{
    collections::HashMap,
    fs,
    io::Read,
};
use flate2::read::ZlibDecoder;

//This function is used to find all the commits of branch (first element of this Vector is the latest commit of this branch)
pub fn get_all_commits_of_branch(branch: &str) -> Vec<String> {
    let refs = get_refs();
    let mut commits = Vec::new();
    // get HEAD commit of branch
    let mut current_hash = match refs.get(branch) {
        Some(hash) => hash.clone(),
        None => {
            println!("Branch not found: {}", branch);
            return commits;
        }
    };

    // traverse commit history
    loop {
        commits.push(current_hash.clone());

        // move to parent
        match get_parent_hash(&current_hash) {
            Some(parent) => current_hash = parent,
            None => break, // reached root commit
        }
    }

    commits
}

// read the content of the commit object
fn read_commit_object(hash: &str) -> String {
    let path = format!(
        ".rvc/objects/{}/{}",
        &hash[..2],
        &hash[2..]
    );

    let file = fs::File::open(path).expect("Commit object not found");
    let mut decoder = ZlibDecoder::new(file);
    let mut content = String::new();
    decoder.read_to_string(&mut content).expect("Failed to read commit");
    content
}

// we are reading the content line by line and checking for the parent hash
fn get_parent_hash(hash: &str) -> Option<String> {
    let content = read_commit_object(hash);
    for line in content.lines() {
        if line.starts_with("parent ") {
            return Some(line[7..].to_string());
        }
    }
    None // no parent, this is the root commit
}

pub fn get_refs() -> HashMap<String, String> {
    let mut refs = HashMap::new();

    if let Ok(entries) = std::fs::read_dir(".rvc/refs/heads") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    refs.insert(name.to_string(), content.trim().to_string());
                }
            }
        }
    }

    refs
}