use serde::{Serialize,Deserialize};

#[derive(Debug,Serialize,Deserialize)]
pub enum RvcMessage{
    NewCommit{
        hash:String,
    },
    GetCommit{
        hash:String,
    },
    CommitData{
        hash:String,
        data:Vec<u8>,
    },
}
