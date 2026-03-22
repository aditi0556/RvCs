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



// use std::{error::Error};
// use futures::StreamExt;
// use tokio::io::{self, AsyncBufReadExt};

// use libp2p::SwarmEvent;

// mod network;
// mod sync;
// mod git;

// use network::swarm::build_swarm;
// use sync::broadcast::broadcast_commit;
// use sync::handler::handle_event;

// #[tokio::main]
// async fn main() -> Result<(), Box<dyn Error>> {

//     // Identity
//     let local_key = libp2p::identity::Keypair::generate_ed25519();

//     // Build swarm
//     let (mut swarm, peer_id, topic) = build_swarm(local_key)?;

//     println!("Node started with Peer ID: {peer_id}");

//     // CLI input
//     let mut stdin = io::BufReader::new(io::stdin()).lines();

//     // MAIN EVENT LOOP
//     loop {
//         tokio::select! {
//             // USER INPUT (CLI)
//             line = stdin.next_line() => {
//                 if let Ok(Some(cmd)) = line {

//                     let parts: Vec<&str> = cmd.split_whitespace().collect();

//                     if parts.is_empty() {
//                         continue;
//                     }

//                     match parts[0] {

//                         // commit <hash>
//                         "commit" => {
//                             if parts.len() < 2 {
//                                 println!("Usage: commit <hash>");
//                                 continue;
//                             }

//                             let hash = parts[1].to_string();

//                             // broadcast commit to network
//                             broadcast_commit(&mut swarm, &topic, hash);
//                         }

//                         // optional: manual fetch
//                         "get" => {
//                             if parts.len() < 2 {
//                                 println!("Usage: get <hash>");
//                                 continue;
//                             }

//                             let hash = parts[1].to_string();

//                             println!("Manual fetch not implemented yet: {}", hash);
//                         }

//                         _ => {
//                             println!("Unknown command");
//                         }
//                     }
//                 }
//             }
//             //  NETWORK EVENTS
//             event = swarm.select_next_some() => {
//                 handle_event(event, &mut swarm);
//             }
//         }
//     }
// }

// // use clap::{Parser, Subcommand};

// // mod commands;
// // mod error;
// // mod objects;

// // #[derive(Parser)]
// // struct Cli {
// //     #[command(subcommand)]
// //     command: Commands,
// // }

// // #[derive(Subcommand)]
// // enum Commands {
// //     Init,
// //     Clone,
// // }

// // fn main() {
// //     let cli = Cli::parse();

// //     let result = match cli.command {
// //         Commands::Init => commands::init(vec![]),
// //         Commands::Clone => commands::clone(vec![]),
// //     };

// //     if let Err(e) = result {
// //         eprintln!("Command error: {}", e);
// //         std::process::exit(1);
// //     }
// // }