use crate::error::GitError;
use crate::objects::{GitObject, Kind};

pub fn cat_file(args: Vec<String>) -> Result<(), GitError> {
    let hex_string = args.last().ok_or(GitError::any("missing object hash"))?;
    if hex_string.len() != 40 {
        return Err(GitError::any("invalid object hash"));
    }
    //return the git object form the hash
    let git_object = GitObject::from_hex_string(hex_string)?;

    match git_object.kind() {
        Kind::Blob => {
            let content = std::str::from_utf8(git_object.contents())?;
            print!("{content}");
            Ok(())
        }
       Kind::Tree => {
        let mut data: &[u8] = git_object.contents();
        while !data.is_empty() {
            // mode until space
            let space_pos = data.iter().position(|&b| b == b' ').unwrap();
            let mode = std::str::from_utf8(&data[..space_pos])?;
            data = &data[space_pos + 1..];

            // filename until null
            let null_pos = data.iter().position(|&b| b == 0).unwrap();
            let filename = std::str::from_utf8(&data[..null_pos])?;
            let hash_bytes = &data[null_pos + 1..null_pos + 21];
            let hash = hex::encode(hash_bytes);
            data = &data[null_pos + 21..];

            println!("{mode} {hash}\t{filename}");
        }
        Ok(())
    }
        Kind::Commit => {
            let content = std::str::from_utf8(git_object.contents())?;
            println!("{content}");
            Ok(())
        }
        kind => Err(GitError::any(format!(
            "support for {} git object not implemented",
            kind,
        ))),
    }
}