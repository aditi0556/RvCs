use crate::error::GitError;
use std::fs;
pub fn init(_args: Vec<String>) -> Result<(), GitError> {
    println!("Aditi sinha");
    fs::create_dir(".rvc")?;
    fs::create_dir(".rvc/objects")?;
    fs::create_dir(".rvc/refs")?;
    fs::write(".rvc/HEAD", "ref: refs/heads/main\n")?;
    println!("Initialized git directory");
    Ok(())
}