use crate::error::GitError;
use crate::objects::GitObject;

//this takes the snapshot of the directory and creates a tree hash
pub fn write_tree(_args: Vec<String>) -> Result<String, GitError> {
    let git_object = match GitObject::from_path("./src", true) {
        Ok(obj) => obj,
        Err(e) => {
            println!("ERROR in from_path: {:?}", e);
            return Err(e);
        }
    };
    let hash=git_object.hex_string();
    println!("Meow {:?}",hash);
    println!("{}", git_object.hex_string());
    Ok(hash)
}

