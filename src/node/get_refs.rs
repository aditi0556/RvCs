use std::{
    collections::{HashMap,HashSet, VecDeque},
};

// LCA using only LOCAL data — called on Peer A after receiving B's commit list
// local_head: Peer A's HEAD, remote_commits: full BFS list of branch Peer B sent
pub fn find_lca(local_head: &str, remote_commits: &[String]) -> Option<String> {
    // put remote commits in a set for O(1) lookup
    let remote_set: HashSet<&String> = remote_commits.iter().collect();

    // BFS from local_head through LOCAL objects only
    // first commit we find that also exists in remote_set is the LCA
    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(local_head.to_string());

    while let Some(hash) = queue.pop_front() {
        if visited.contains(&hash) { continue; }
        visited.insert(hash.clone());

        if remote_set.contains(&hash) {
            return Some(hash); // nearest common ancestor
        }

        // walk LOCAL parents only — no network needed
        for parent in get_all_parent_hashes(&hash) {
            queue.push_back(parent);
        }
    }
    None
}


//This function is used to find all the commits of branch (first element of this Vector is the latest commit of this branch)
pub fn get_all_commits_of_branch(branch: &str) -> Vec<String> {
    println!("Branch name is {}",branch);
     let head = read_ref(branch);

    if head.is_empty() {
        println!("Branch not found: {}", branch);
        return vec![];
    }

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue = vec![head];
    let mut commits = Vec::new();

    // BFS — handles merge commits with two parents correctly
    while let Some(hash) = queue.pop() {
        if visited.contains(&hash) { continue; }
        visited.insert(hash.clone());
        commits.push(hash.clone());

        // get ALL parents (merge commits have 2)
        for parent in get_all_parent_hashes(&hash) {
            if !visited.contains(&parent) {
                queue.push(parent);
            }
        }
    }
    commits
}

// you need this — returns ALL parents not just first
pub fn get_all_parent_hashes(hash: &str) -> Vec<String> {
    let path = format!(".git/objects/{}/{}", &hash[..2], &hash[2..]);
    let Ok(raw) = std::fs::read(&path) else { return vec![] };
    let Ok(dec) = crate::node::event_loop::decompress_zlib(&raw) else { return vec![] };
    let Some(np) = dec.iter().position(|&b| b == 0) else { return vec![] };
    let Ok(text) = std::str::from_utf8(&dec[np+1..]) else { return vec![] };
    let mut parents = Vec::new();
    for line in text.lines() {
        if line.is_empty() { break; }
        if line.starts_with("parent ") {
            parents.push(line[7..].trim().to_string());
        }
    }
    parents
}

pub fn read_ref(branch: &str) ->String{
    std::fs::read_to_string(format!(".git/refs/heads/{}",branch))
      .unwrap_or_default().trim().to_string()
}

pub fn write_ref(branch: &str, hash: &str) {
    let path = format!(".git/refs/heads/{}", branch);
    
    // create the directory if it doesn't exist
    let dir = format!(".git/refs/heads");
    std::fs::create_dir_all(&dir).unwrap();
    
    std::fs::write(&path, hash).unwrap();
}

// it returns HashMap of <branch_name,hash>
pub fn get_refs() -> HashMap<String, String> {
    let mut refs = HashMap::new();

    let base = std::path::Path::new(".git/refs/heads");

    if let Ok(entries) = std::fs::read_dir(base) {
        for entry in entries.flatten() {
            let path = entry.path();

            // Only process files
            if !path.is_file() {
                continue;
            }

            // Extract branch name only
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let hash = content.trim(); // remove newline safely

                    // Extra safety: skip empty hashes
                    if !hash.is_empty() {
                        refs.insert(name.to_string(), hash.to_string());
                    }
                }
            }
        }
    }

    refs
}