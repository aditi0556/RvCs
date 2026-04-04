use crate::objects::GitObject;
use crate::error::GitError;
use std::path::Path;


pub fn add_paths(paths: &[String]) -> Result<(), GitError> {
    if paths.is_empty() {
        return Err(GitError::any("nothing specified, nothing added"));
    }

    let mut errors = Vec::new();

    for filepath in paths {
        let path = std::path::Path::new(filepath);

        if !path.exists() {
            errors.push(format!("pathspec '{}' did not match any files", filepath));
            continue;
        }

        let result = if path.is_dir() {
            add_directory(filepath)
        } else if path.is_file() {
            add(filepath)
        } else {
            Err(GitError::any(format!("unsupported path type: {}", filepath)))
        };

        if let Err(e) = result {
            errors.push(format!("{}: {}", filepath, e));
        }
    }

    if !errors.is_empty() {
        return Err(GitError::any(errors.join("\n")));
    }

    Ok(())
}

// ─── Index entry ─────────────────────────────────────────────────────────────
// .git/index format (one line per staged file):
// <mode> <blob_hash> <filepath>
// e.g: 100644 a1b2c3d4e5f6... src/main.rs

pub fn add(filepath: &str) -> Result<(), GitError> {
    let path = Path::new(filepath);

    if !path.exists() {
        return Err(GitError::any(format!("pathspec '{}' did not match any files", filepath)));
    }

    // use your existing GitObject::from_path with write=true
    // this creates the blob object and writes it to .git/objects/
    let git_object = GitObject::from_path(path, true)?;

    let blob_hash = git_object.hex_string();

    // determine mode same way your from_path does
    let mode = {
        let meta = std::fs::metadata(path)?;
        if meta.is_dir() {
            // staging a directory — recursively add all files inside
            return add_directory(filepath);
        } else if meta.is_symlink() {
            "120000"
        } else {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if meta.permissions().mode() & 0o111 != 0 { "100755" } else { "100644" }
            }
            #[cfg(not(unix))]
            { "100644" }
        }
    };

    // read existing index, remove old entry for this path, add new one
    stage_file(mode, &blob_hash, filepath)?;

    println!("Staged: {}", filepath);
    Ok(())
}

// recursively add all files in a directory
fn add_directory(dirpath: &str) -> Result<(), GitError> {
    let ignored = [".git"];
    for entry in std::fs::read_dir(dirpath)? {
        let entry = entry?;
        let filename = entry.file_name().into_string()
            .map_err(|_| GitError::any("invalid filename"))?;

        if ignored.contains(&filename.as_str()) { continue; }

        let full_path = format!("{}/{}", dirpath, filename);
        let meta = entry.metadata()?;

        if meta.is_dir() {
            add_directory(&full_path)?;
        } else {
            add(&full_path)?;
        }
    }
    Ok(())
}

// write a single file entry into .git/index
fn stage_file(mode: &str, blob_hash: &str, filepath: &str) -> Result<(), GitError> {
    let index_path = ".git/index";
    let existing = std::fs::read_to_string(index_path).unwrap_or_default();

    // remove any existing entry for this filepath (handles re-staging)
    let mut entries: Vec<String> = existing
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter(|l| {
            // each line: "<mode> <hash> <path>"
            let mut parts = l.splitn(3, ' ');
            parts.nth(2).map(|p| p != filepath).unwrap_or(true)
        })
        .map(|l| l.to_string())
        .collect();

    // append new entry
    entries.push(format!("{} {} {}", mode, blob_hash, filepath));

    std::fs::write(index_path, entries.join("\n") + "\n")
        .map_err(|e| GitError::any(format!("cannot write index: {}", e)))?;

    Ok(())
}

//  get_staged_files 
// returns Vec of (mode, blob_hash, filepath)
pub fn get_staged_files() -> Vec<(String, String, String)> {
    let Ok(content) = std::fs::read_to_string(".git/index") else {
        return vec![];
    };

    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|line| {
            let mut parts = line.splitn(3, ' ');
            let mode      = parts.next()?.to_string();
            let blob_hash = parts.next()?.to_string();
            let filepath  = parts.next()?.to_string();
            Some((mode, blob_hash, filepath))
        })
        .collect()
}

// clear index after commit 
pub fn clear_index() -> Result<(), GitError> {
    std::fs::write(".git/index", "")
        .map_err(|e| GitError::any(format!("cannot clear index: {}", e)))?;
    Ok(())
}