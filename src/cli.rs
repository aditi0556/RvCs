use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "rvcd")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Join {
        #[arg(short, long)]
        port: u16,
    },
    // Discover,
}