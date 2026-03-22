mod cli;
mod node;

use clap::Parser;

#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();
    match cli.command {
        cli::Commands::Join {port } => {
            node::start_node(port).await
        },
    }
}



