use std::{
    collections::HashMap,
};


pub fn get_refs() -> HashMap<String, String> {
    let mut refs = HashMap::new();

    if let Ok(entries) = std::fs::read_dir(".git/refs/heads") {
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