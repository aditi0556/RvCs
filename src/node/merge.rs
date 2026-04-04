use crate::node::get_refs::{
    get_all_parent_hashes,
    write_ref,
    read_ref,
};
use std;
use crate::node::diff3::diff3_merge;
use std::collections::{HashMap};
use std::io::Read;
use crate::node::event_loop::decompress_zlib;

//returns mapping of all the files (recursively traverse all the trees)
fn walk_tree(tree_hash: &str, prefix: &str, out: &mut HashMap<String, String>) {
    use std::io::BufRead;
    let path = format!(".git/objects/{}/{}", &tree_hash[..2], &tree_hash[2..]);
    let Ok(raw) = std::fs::read(&path) else { return };
    let Ok(dec) = decompress_zlib(&raw) else { return };
    let Some(np) = dec.iter().position(|&b| b == 0) else { return };
    let content = &dec[np+1..];
    let mut cursor = std::io::Cursor::new(content);
    loop {
        let mut mode = Vec::new();
        let mut filename = Vec::new();
        let mut raw_hash = vec![0u8; 20];
        if cursor.read_until(b' ', &mut mode).unwrap_or(0) == 0 { break; }
        if cursor.read_until(b'\0', &mut filename).unwrap_or(0) == 0 { break; }
        if cursor.read_exact(&mut raw_hash).is_err() { break; }
        mode.pop(); filename.pop();
        let entry_hash: String = raw_hash.iter().map(|b| format!("{:02x}", b)).collect();
        let Ok(name) = String::from_utf8(filename) else { break };
        let Ok(mode_str) = std::str::from_utf8(&mode) else { break };
        let full_path = if prefix.is_empty() { name.clone() } else { format!("{}/{}", prefix, name) };
        match mode_str {
            "40000"  => walk_tree(&entry_hash, &full_path, out),
            "100644" | "100755" | "120000" => { out.insert(full_path, entry_hash); }
            _ => {}
        }
        if cursor.position() as usize >= content.len() { break; }
    }
}

//returns the actual content of a blob object -- line by line
fn read_blob_lines(blob_hash: &str) -> Vec<String> {
    let path = format!(".git/objects/{}/{}", &blob_hash[..2], &blob_hash[2..]);
    let Ok(raw) = std::fs::read(&path) else { return vec![] };
    let Ok(dec) = decompress_zlib(&raw) else { return vec![] };
    let Some(np) = dec.iter().position(|&b| b == 0) else { return vec![] };
    let Ok(text) = std::str::from_utf8(&dec[np+1..]) else { return vec![] };
    text.lines().map(|l| l.to_string()).collect()
}

//takes blob hash and rewrite it to a position
fn write_blob_to_worktree(path: &str, blob_hash: &str) {
    let obj_path = format!(".git/objects/{}/{}", &blob_hash[..2], &blob_hash[2..]);
    let Ok(raw) = std::fs::read(&obj_path) else { return };
    let Ok(dec) = decompress_zlib(&raw) else { return };
    let Some(np) = dec.iter().position(|&b| b == 0) else { return };
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(path, &dec[np+1..]).ok();
    println!("write_blob_to_worktree");
}

//returns the <file-path,blob_hash>
fn list_files_at_commit(commit_hash: &str) -> HashMap<String, String> {
    let mut files = HashMap::new();
    let path = format!(".git/objects/{}/{}", &commit_hash[..2], &commit_hash[2..]);
    //read file at that path
    let Ok(raw) = std::fs::read(&path) else { return files };
    //decompress the stored content
    let Ok(dec) = decompress_zlib(&raw) else { return files };
    //skip the header <objectType><size>/0<content>
    let Some(np) = dec.iter().position(|&b| b == 0) else { return files };
    //reading the content and converts form utf-8 to valid string typr &str
    let Ok(text) = std::str::from_utf8(&dec[np+1..]) else { return files };
    if let Some(tree_hash) = text.lines().find(|l| l.starts_with("tree ")).map(|l| l[5..].trim().to_string()) {
        walk_tree(&tree_hash, "", &mut files);
    }
    files
}

fn checkout_tree(commit_hash: &str) {
    println!("Checkout tree");
    for (path, blob_hash) in list_files_at_commit(commit_hash) {
        println!("Checkout yes");
        write_blob_to_worktree(&path, &blob_hash);
    }
}

pub fn merge_branch(branch: &str, remote_head: &str) {
    let local_head = read_ref(branch);

    if local_head.is_empty() {
        // branch doesn't exist locally yet — just set the ref and checkout
        println!("New branch, checking out {}", &remote_head[..7]);
        checkout_tree(remote_head);
        write_ref(branch, remote_head);
        return;
    }

    if local_head == remote_head {
        println!("Already up to date.");
        return;
    }

    // fast-forward check: walk LOCAL history only
    // if remote_head appears in our local ancestors, remote is behind us
    let local_ancestors = collect_ancestors(&local_head);
    if local_ancestors.contains(remote_head) {
        println!("Already up to date (remote is behind local).");
        return;
    }

    // check if we are behind remote — walk remote history (objects now on disk)
    // remote objects were just written to disk so get_all_parent_hashes works
    let remote_ancestors = collect_ancestors(remote_head);
    if remote_ancestors.contains(&local_head) {
        println!("Fast-forward: {} → {}", &local_head[..7], &remote_head[..7]);
        checkout_tree(remote_head);
        write_ref(branch, remote_head);
        return;
    }

    // diverged — find LCA now that remote objects are on disk
    // walk remote ancestors, first one found in local_ancestors is LCA
    let lca = remote_ancestors.iter()
        .find(|h| local_ancestors.contains(*h))
        .cloned();

    //changing Option<String> to String
    let lca = match lca {
        Some(l) => { 
            println!("Merge base: {}", &l[..7]);
            l //return l 
        }
        None => {
            println!("No common ancestor — unrelated histories.");
            return;
        }
    };

    // three-way merge
    let base_files   = list_files_at_commit(&lca);
    let local_files  = list_files_at_commit(&local_head);
    let remote_files = list_files_at_commit(remote_head);

    let all_paths: std::collections::HashSet<String> = base_files.keys()
        .chain(local_files.keys())
        .chain(remote_files.keys())
        .cloned()
        .collect();

    let mut conflicts: Vec<String> = Vec::new();

    for path in &all_paths {
        //if any blob is deleted then there it returns None
        //returns the blobs at that path
        let base_blob   = base_files.get(path);
        let local_blob  = local_files.get(path);
        let remote_blob = remote_files.get(path);

        match (base_blob, local_blob, remote_blob) {
            (b, l, r) if l == b && r == b => {}
            //since local did not changed it is same as the base blob  on which remote blob file is made therefore there is no need to check for conflicts here
            //so here if the remote_blob is none then just delete that file after merging
            (b, l, r) if l == b && r != b => {
                match r {
                    Some(h) => { write_blob_to_worktree(path, h); println!("Updated:    {}", path); }
                    None    => { std::fs::remove_file(path).ok(); println!("Deleted:    {}", path); }
                }
            }
            (b, l, r) if r == b && l != b => { println!("Kept local: {}", path); }

            (_, l, r) if l == r            => { println!("Converged:  {}", path); }
            _ => {
                
                let base_lines   = base_blob.map(|h| read_blob_lines(h)).unwrap_or_default();
                let local_lines  = local_blob.map(|h| read_blob_lines(h)).unwrap_or_default();
                let remote_lines = remote_blob.map(|h| read_blob_lines(h)).unwrap_or_default();
                let (merged, has_conflict) = diff3_merge(&base_lines, &local_lines, &remote_lines);
                if let Some(parent) = std::path::Path::new(path).parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                std::fs::write(path, merged.join("\n")).ok();
                if has_conflict {
                    println!("CONFLICT:   {}", path);
                    conflicts.push(path.clone());
                } else {
                    println!("Merged:     {}", path);
                }
            }
        }
    }

    if conflicts.is_empty() {
        write_ref(branch, remote_head);
        println!("Merge successful → {}", &remote_head[..7]);
    } else {
        println!("Merge failed — {} conflict(s). Fix and commit:", conflicts.len());
        std::fs::write(".git/MERGE_HEAD",remote_head).unwrap();
        std::fs::write(".git/MERGE_MSG",format!("Merge branch '{}' from remote",branch)).unwrap();
        for f in &conflicts { println!("  CONFLICT: {}", f); }
    }
}

// walks LOCAL commit graph only — no network
fn collect_ancestors(start: &str) -> std::collections::HashSet<String> {
    let mut visited = std::collections::HashSet::new();
    let mut queue = std::collections::VecDeque::new();
    queue.push_back(start.to_string());
    while let Some(h) = queue.pop_front() {
        if visited.contains(&h) { continue; }
        visited.insert(h.clone());
        for p in get_all_parent_hashes(&h) { queue.push_back(p); }
    }
    visited
}