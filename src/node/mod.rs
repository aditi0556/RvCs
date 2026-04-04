pub mod behaviour;
pub mod event_loop;
pub mod state;
pub mod protocol;
pub mod command;
pub mod objects;
pub mod diff3;
pub mod merge;
pub mod get_refs;
use tokio::sync::mpsc::Sender;
use libp2p::{
    PeerId,
    identity,
    noise,
    yamux,
    Multiaddr,
    StreamProtocol,
    request_response::{self,ProtocolSupport},
};

use behaviour::RvcBehaviour;
use state::AppState;

use std::{sync::{Arc, Mutex}, time::Duration};

use tokio::sync::mpsc;
use crate::node::{
    command::Command
};

pub async fn start_node(port: u16) {
    let key = identity::Keypair::generate_ed25519();
    let peer_id = key.public().to_peer_id();

    println!("Peer ID: {}", peer_id);

    let mdns = libp2p::mdns::tokio::Behaviour::new(
        Default::default(),
        peer_id,
    ).unwrap();

    let req_res = request_response::Behaviour::new(
        [(StreamProtocol::new("/git/1.0.0"), ProtocolSupport::Full)],
        request_response::Config::default(),
    );

    let behaviour = RvcBehaviour { mdns, req_res };

    let mut swarm = libp2p::SwarmBuilder::with_existing_identity(key)
    .with_tokio()
    .with_tcp(
        Default::default(),
        noise::Config::new,
        yamux::Config::default,
    )
    .expect("tcp transport failed")  // ← unwrap the Result here
    .with_behaviour(|_| Ok(behaviour))  // ← behaviour also needs to return Result
    .expect("behaviour failed")
    .with_swarm_config(|cfg| {
        cfg.with_idle_connection_timeout(Duration::from_secs(300))
    })
    .build();

    let addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", port)
        .parse()
        .unwrap();
    swarm.listen_on(addr).unwrap();
    println!("Listening on port {}", port);
    //here this default() creates a new and empty state
    let state = Arc::new(Mutex::new(AppState::default()));

    //Here tx.Send().await returns  Result<>,Err>
    //Here rx.recv.await returns an Option<T>
    let (tx, rx) = mpsc::channel(32);

    // spawn event loop
    let state_clone = state.clone();
    tokio::spawn(async move {
        crate::node::event_loop::create_event_loop(swarm, state_clone, rx).await;
    });
    // cli loop
    cli_loop(tx).await;
}


async fn cli_loop(tx: Sender<Command>) {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let mut lines = BufReader::new(tokio::io::stdin()).lines();

    println!("Commands: rvcd discover");
    println!("Commands: rvcd join peerID multiaddr");
    println!("Commands: rvcd branches <branchID>");

  while let Ok(Some(line)) = lines.next_line().await {
    let parts: Vec<&str> = line.trim().split_whitespace().collect();

    if parts.is_empty() {
        continue;
    }

    match parts[0] {
        "rvcd" => {
            if parts.len() < 2 {
                println!("Usage: gitd <command>");
                continue;
            }

            match parts[1] {
                "discover" => {
                    tx.send(Command::Discover).await.unwrap();
                }

                "merge"=>{
                    if parts.len() != 4{
                        println!("Usage: gitd merge <peerId> <branchName>");
                        continue;
                    }
                    let peer: PeerId = match parts[2].parse() {
                        Ok(p) => p,
                        Err(_) => {
                            println!("Invalid peer_id");
                            continue;
                        }
                    };
                    let branch= parts[3].to_string();
                    tx.send(Command::Merge { peer, branch }).await.unwrap();
                }

                "branches" => {
                    if parts.len() != 3 {
                        println!("Usage: gitd branches <peer_id>");
                        continue;
                    }

                    let peer: PeerId = match parts[2].parse() {
                        Ok(p) => p,
                        Err(_) => {
                            println!("Invalid peer_id");
                            continue;
                        }
                    };

                    tx.send(Command::Branches { peer }).await.unwrap();
                }

                "dial" => {
                    if parts.len() != 4 {
                        println!("Usage: gitd dial <peer_id> <addr>");
                        continue;
                    }

                    let peer: PeerId = match parts[2].parse() {
                        Ok(p) => p,
                        Err(_) => {
                            println!("Invalid peer_id");
                            continue;
                        }
                    };

                    let addr: Multiaddr = match parts[3].parse() {
                        Ok(a) => a,
                        Err(_) => {
                            println!("Invalid multiaddr");
                            continue;
                        }
                    };

                    tx.send(Command::Dial { peer, addr }).await.unwrap();
                }

                _ => println!("Unknown rvcd command"),
            }
        }

        _ => println!("Unknown command"),
    }
}}