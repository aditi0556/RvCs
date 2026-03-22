use crate::node::{
    behaviour::{RvcBehaviour, RvcBehaviourEvent},
    command::Command,
    protocol::{RvcRequest, RvcResponse},
    state::AppState,
};
use futures::StreamExt;
use libp2p::{
    mdns,
    request_response::{Event as ReqResEvent, Message},
    swarm::{Swarm, SwarmEvent},
};

use std::{
    sync::{Arc, Mutex}
};
use tokio::sync::mpsc::Receiver;

pub async fn run_event_loop(
    mut swarm: Swarm<RvcBehaviour>,
    state: Arc<Mutex<AppState>>,
    mut rx: Receiver<Command>,
) {
    loop {
        tokio::select! {
            // cli commands 
            Some(cmd) = rx.recv() => {
                match cmd {
                    Command::Discover => {
                        println!(" Discovering peers...");

                        let peers: Vec<_> = {let st = state.lock().unwrap();st.peers.keys().cloned().collect()};

                        if peers.is_empty() {
                            println!(" No peers known yet (wait for mDNS)");
                        }
                        for peer in peers {
                            swarm.behaviour_mut().req_res.send_request(
                                &peer,
                                RvcRequest(b"GET_PEERS".to_vec())
                            );
                        }
                    },
                    Command::Dial { peer, addr } => {
                        println!("Dialing {} at {}", peer, addr);
                        swarm.add_peer_address(peer, addr.clone());
                        if let Err(e) = swarm.dial(addr) {
                            println!("Dial failed: {:?}", e);
                        }
                    }
                    
                }
            }

            // network events
            event = swarm.select_next_some() => {
                println!("Event is {:?}",event);
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        println!("Listening on {address}");
                    }

                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        println!("Connected to {}", peer_id);
                        swarm.behaviour_mut().req_res.send_request(
                            &peer_id,
                            RvcRequest(b"GET_PEERS".to_vec())
                        );
                    }

                    // add peers here (if missing)
                    SwarmEvent::Behaviour(RvcBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                        let mut st = state.lock().unwrap();

                        for (peer, addr) in list {
                            println!(" Discovered {} at {}", peer, addr);
                            swarm.add_peer_address(peer, addr.clone());
                            st.peers
                            .entry(peer)
                            .or_insert_with(Vec::new)
                            .push(addr);
                        }
                    }

                    // remove peers
                     SwarmEvent::Behaviour(RvcBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                        let mut st = state.lock().unwrap();
                        for (peer, addr) in list {
                            println!(" Expired {} at {}", peer, addr);
                            if let Some(addrs) = st.peers.get_mut(&peer) {
                                addrs.retain(|a| a != &addr);
                                if addrs.is_empty() {
                                    st.peers.remove(&peer);
                                }
                            }
                        }
                    }

                    // request response
                    SwarmEvent::Behaviour(RvcBehaviourEvent::ReqRes(e)) => {
                        match e {

                            // incoming request
                            ReqResEvent::Message {message,..} => match message {

                                Message::Request { request, channel, .. } => {

                                    if request.0 == b"GET_PEERS" {
                                        let peers = state.lock().unwrap().peers.clone();

                                        let list = peers.iter()
                                            .map(|(peer, _)| peer.to_string())
                                            .collect::<Vec<_>>()
                                            .join(",");

                                        let response = RvcResponse(list.into_bytes());

                                        swarm.behaviour_mut()
                                            .req_res
                                            .send_response(channel, response)
                                            .unwrap();
                                    }
                                }

                                // response received
                                 Message::Response { response, .. } => {
                                    let data = String::from_utf8(response.0).unwrap();

                                    let mut st = state.lock().unwrap();
                                    for p in data.split(",") {
                                        if let Ok(peer_id) = p.parse() {
                                            st.peers.entry(peer_id).or_insert_with(Vec::new);
                                        }
                                    }

                                    println!("\n All Known Peers:");
                                    for (peer, addrs) in st.peers.iter() {
                                        println!("{} -> {:?}", peer, addrs);
                                    }
                                }
                            },

                            _ => {}
                        }
                    }

                    _ => {}
                }
            }
        }
    }
}





