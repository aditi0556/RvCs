use std::fs;

pub fn write_object(hash: &str, data: &[u8]) {
    let path = format!(".git/objects/{hash}");
    let _ = fs::create_dir_all(".git/objects");
    fs::write(path, data).unwrap();
}

pub fn read_object(hash: &str) -> Vec<u8> {
    let path = format!(".git/objects/{hash}");
    fs::read(path).unwrap_or_default()
}

pub fn has_object(hash: &str) -> bool {
    let path = format!(".git/objects/{hash}");
    std::path::Path::new(&path).exists()
}