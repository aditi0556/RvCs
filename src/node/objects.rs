use std::io::Read;
use crate::node::event_loop::decompress_zlib;

pub fn collect_all_objects(hash: &str, out: &mut Vec<String>) {
    if out.contains(&hash.to_string()) {
        return;
    }
    out.push(hash.to_string());

    let path = format!(".git/objects/{}/{}", &hash[..2], &hash[2..]);
    let Ok(raw) = std::fs::read(&path) else { return };

    // decompress (assuming zlib like git)
    let Ok(decompressed) = decompress_zlib(&raw) else { return };

    // object header format: "<type> <size>\0<content>"
    let Some(null_pos) = decompressed.iter().position(|&b| b == 0) else { return };
    let header = &decompressed[..null_pos];
    let content = &decompressed[null_pos + 1..];

    let Ok(header_str) = std::str::from_utf8(header) else { return };

    if header_str.starts_with("commit") {
        // commit content is text: find "tree <hash>\n"
        let Ok(text) = std::str::from_utf8(content) else { return };
        for line in text.lines() {
            if line.starts_with("tree ") {
                let tree_hash = line[5..].trim();
                collect_all_objects(tree_hash, out);
            }
            else if line.starts_with("parent ") {
                let parent_hash = line[7..].trim();
                collect_all_objects(parent_hash, out);
            }
            // stop at blank line (start of commit message)
            if line.is_empty() { break; }
        }
    } else if header_str.starts_with("tree") {
        // binary format: "<mode> <filename>\0<20 raw bytes>"
        let mut cursor = std::io::Cursor::new(content);
        loop {
            let mut mode = Vec::new();
            let mut filename = Vec::new();
            let mut raw_hash = vec![0u8; 20];

            use std::io::BufRead;
            if cursor.read_until(b' ', &mut mode).unwrap_or(0) == 0 { break; }
            if cursor.read_until(b'\0', &mut filename).unwrap_or(0) == 0 { break; }
            if cursor.read_exact(&mut raw_hash).is_err() { break; }

            // convert 20 raw bytes → 40 char hex string
            let entry_hash = raw_hash.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();

            mode.pop(); // remove trailing space
            let Ok(mode_str) = std::str::from_utf8(&mode) else { break };

            match mode_str {
                "40000" => {
                    // subtree — recurse
                    collect_all_objects(&entry_hash, out);
                }
                "100644" | "100755" | "120000" => {
                    // blob — just add the hash, no need to recurse
                    if !out.contains(&entry_hash) {
                        out.push(entry_hash);
                    }
                }
                _ => {}
            }

            if cursor.position() as usize >= content.len() { break; }
        }
    }
    // blobs: no children to walk
}
