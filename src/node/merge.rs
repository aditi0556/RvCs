use std::collections::{
    HashMap,HashSet,
};
use crate::node::event_loop::decompress_zlib;
use std::io::Read;

pub fn merge_branch(branch: &str, remote_head: &str) -> Vec<String> {
    let local_head = read_ref(branch);

    if local_head == remote_head {
        println!("Already up to date.");
        return vec![];
    }

    // find lowest common ancestor
    let base = find_lca(&local_head, remote_head);

    let base_files   = base.as_deref().map(|h| list_files_at_commit(h)).unwrap_or_default();
    let local_files  = list_files_at_commit(&local_head);
    let remote_files = list_files_at_commit(remote_head);

    // collect all paths across all three snapshots
    let all_paths: HashSet<String> = base_files.keys()
        .chain(local_files.keys())
        .chain(remote_files.keys())
        .cloned()
        .collect();

    let mut conflicts = Vec::new();

    for path in &all_paths {
        let base_hash   = base_files.get(path).map(|s| s.as_str());
        let local_hash  = local_files.get(path).map(|s| s.as_str());
        let remote_hash = remote_files.get(path).map(|s| s.as_str());

        match (base_hash, local_hash, remote_hash) {

            // only remote changed → apply it
            (b, l, r) if l == b && r != b => {
                if let Some(hash) = remote_hash {
                    write_blob_to_worktree(&path, hash);
                    println!("Updated: {}", path);
                } else {
                    std::fs::remove_file(&path).ok();
                    println!("Deleted: {}", path);
                }
            }

            // only local changed → keep it, nothing to do
            (b, l, r) if r == b && l != b => {
                println!("Kept local: {}", path);
            }

            // both unchanged → nothing to do
            (b, l, r) if l == b && r == b => {}

            // both changed to the same thing → fine
            (_, l, r) if l == r => {}

            // both changed differently → conflict
            _ => {
                println!("CONFLICT: {}", path);
                conflicts.push(path.clone());
            }
        }
    }

    if conflicts.is_empty() {
        write_ref(branch, remote_head);
        println!("Merge successful → {}", remote_head);
    } else {
        println!("\nMerge failed — {} file(s) in conflict:", conflicts.len());
        for f in &conflicts {
            println!("  ✗ {}", f);
        }
    }

    conflicts
}

// walks commit parents to find lowest common ancestor
fn find_lca(a: &str, b: &str) -> Option<String> {
    let ancestors_a = commit_ancestors(a);
    let set_a: HashSet<String> = ancestors_a.into_iter().collect();
    for commit in commit_ancestors(b) {
        if set_a.contains(&commit) {
            return Some(commit);
        }
    }
    None
}

fn commit_ancestors(start: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut queue = vec![start.to_string()];
    while let Some(hash) = queue.pop() {
        if result.contains(&hash) { continue; }
        result.push(hash.clone());
        // read parent lines from commit object
        let path = format!(".rvc/objects/{}/{}", &hash[..2], &hash[2..]);
        let Ok(raw) = std::fs::read(&path) else { continue };
        let Ok(dec) = decompress_zlib(&raw) else { continue };
        let Some(np) = dec.iter().position(|&b| b == 0) else { continue };
        let Ok(text) = std::str::from_utf8(&dec[np+1..]) else { continue };
        for line in text.lines() {
            if line.is_empty() { break; }
            if line.starts_with("parent ") {
                queue.push(line[7..].trim().to_string());
            }
        }
    }
    result
}

fn write_ref(branch: &str, hash: &str) {
    let path = format!(".rvc/refs/heads/{}", branch);
    std::fs::write(path, hash).unwrap();
}

fn read_ref(branch: &str) -> String {
    let path = format!(".rvc/refs/heads/{}", branch);
    std::fs::read_to_string(path).unwrap_or_default().trim().to_string()
}

fn write_blob_to_worktree(path: &str, blob_hash: &str) {
    let obj_path = format!(".rvc/objects/{}/{}", &blob_hash[..2], &blob_hash[2..]);
    let Ok(raw) = std::fs::read(&obj_path) else { return };
    let Ok(dec) = decompress_zlib(&raw) else { return };
    // skip "blob <size>\0" header
    let Some(np) = dec.iter().position(|&b| b == 0) else { return };
    let content = &dec[np+1..];
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(path, content).unwrap();
}

pub fn list_files_at_commit(commit_hash: &str) -> HashMap<String, String> {
    let mut files = HashMap::new();
    let path = format!(".rvc/objects/{}/{}", &commit_hash[..2], &commit_hash[2..]);
    let Ok(raw) = std::fs::read(&path) else { return files };
    let Ok(decompressed) = decompress_zlib(&raw) else { return files };
    let Some(null_pos) = decompressed.iter().position(|&b| b == 0) else { return files };
    let content = &decompressed[null_pos + 1..];
    let Ok(text) = std::str::from_utf8(content) else { return files };

    let tree_hash = text.lines()
        .find(|l| l.starts_with("tree "))
        .map(|l| l[5..].trim().to_string());

    if let Some(tree_hash) = tree_hash {
        walk_tree(&tree_hash, "", &mut files);
    }
    files
}

pub fn walk_tree(tree_hash: &str, prefix: &str, out: &mut HashMap<String, String>) {
    let path = format!(".rvc/objects/{}/{}", &tree_hash[..2], &tree_hash[2..]);
    let Ok(raw) = std::fs::read(&path) else { return };
    let Ok(decompressed) = decompress_zlib(&raw) else { return };
    let Some(null_pos) = decompressed.iter().position(|&b| b == 0) else { return };
    let content = &decompressed[null_pos + 1..];

    let mut cursor = std::io::Cursor::new(content);
    loop {
        let mut mode = Vec::new();
        let mut filename = Vec::new();
        let mut raw_hash = vec![0u8; 20];

        use std::io::BufRead;
        if cursor.read_until(b' ', &mut mode).unwrap_or(0) == 0 { break; }
        if cursor.read_until(b'\0', &mut filename).unwrap_or(0) == 0 { break; }
        if cursor.read_exact(&mut raw_hash).is_err() { break; }

        mode.pop();
        filename.pop();

        let entry_hash = raw_hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        let Ok(name) = String::from_utf8(filename) else { break };
        let Ok(mode_str) = std::str::from_utf8(&mode) else { break };

        let full_path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", prefix, name)
        };

        match mode_str {
            "40000" => walk_tree(&entry_hash, &full_path, out),
            "100644" | "100755" | "120000" => { out.insert(full_path, entry_hash); }
            _ => {}
        }

        if cursor.position() as usize >= content.len() { break; }
    }
}