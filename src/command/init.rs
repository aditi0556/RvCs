use crate::error::GitError;
use std::fs;
use std::fs::File;

pub fn init(_args: Vec<String>) -> Result<(), GitError> {
    println!("Aditi sinha");
    fs::create_dir(".git")?;
    fs::create_dir(".git/objects")?;
    fs::create_dir(".git/refs")?;
    File::create(".git/index")?; //stores the files which are staged <mode><filename><blob>
    fs::write(".git/HEAD", "ref: refs/heads/main\n")?;
    println!("Initialized git directory");
    Ok(())
}