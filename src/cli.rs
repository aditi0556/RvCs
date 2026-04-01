use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rvcd")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    // Network
    Join {
        #[arg(short, long)]
        port: u16,
    },

    // Git-like
    Init{
        #[arg()]
        args:Vec<String>,
    },
    Commit {
        #[arg(short, long)]
        message: String,
    },

    Clone {
        repo: String,
    },

    CatFile {
        args: Vec<String>,
    },

    HashObject {
        args: Vec<String>,
    },

    LsTree {
        args: Vec<String>,
    },
    CreateBranch {
        name:String,
    },
    SwitchBranch{
        name:String,
    },
    WriteTree,
    CommitTree {
        args: Vec<String>,
    },
}