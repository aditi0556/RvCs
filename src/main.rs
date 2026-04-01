mod cli;
use clap::Parser;
mod command;
mod node;
pub mod error;
pub mod objects;
#[tokio::main]
async fn main() {
    let cli = cli::Cli::parse();
    match cli.command {
        cli::Commands::Join { port } => node::start_node(port).await,
        cli::Commands::Init { args } => {
            let _=command::init::init(args);
        },
        cli::Commands::Commit { message} =>{
           let _= command::commit::commit(message);
        },
         cli::Commands::Clone { repo } => {
            let _=command::clone::clone(vec![repo]);
        },
        cli::Commands::CatFile { args } => {
           let _=command::cat_file::cat_file(args);
        },
        cli::Commands::HashObject { args } => {
            let _=command::hash_objects::hash_object(args);
        },
        cli::Commands::LsTree { args } => {
            let _=command::ls_trees::ls_tree(args);
        },
        cli::Commands::WriteTree => {
            let _=command::write_tree::write_tree(vec![]);
        },
        cli::Commands::CommitTree { args } => {
            let _=command::commit_tree::commit_tree(args);
        },
        cli::Commands::CreateBranch { name }=>{
            let _=command::refs::create_branch(&name);
        },
        cli::Commands::SwitchBranch { name}=>{
            let _=command::refs::switch_branch(&name);
        } 
    }
}
