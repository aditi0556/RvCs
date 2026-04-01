use crate::node::{
    behaviour::{RvcBehaviour, RvcBehaviourEvent},
    command::Command,
    get_refs::get_refs,
    protocol::{RvcRequest, RvcResponse},
    state::AppState,
};
use futures::StreamExt;
use libp2p::{
    mdns,
    request_response::{Event as ReqResEvent, Message},
    swarm::{Swarm, SwarmEvent},
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Receiver;
pub async fn create_event_loop(
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
                    Command::Branches {peer} => {
                        println!("Give branches for this {}",peer);
                        swarm.behaviour_mut().req_res.send_request(&peer,RvcRequest(b"GET_REFS".to_vec()) );
                    }
                }
            }

            // network events
            event = swarm.select_next_some() => {
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
                    // add peers here (if missing)
                    SwarmEvent::Behaviour(RvcBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                        let mut st = state.lock().unwrap();

                        for (peer, addr) in list {
                            // Add peer to swarm
                            swarm.add_peer_address(peer, addr.clone());

                            // Add to state
                            st.peers
                                .entry(peer)
                                .or_insert_with(Vec::new)
                                .push(addr.clone());

                            // Print in two lines
                            println!("Peer ID: {}", peer);
                            println!("Address: {}\n", addr);
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
                            ReqResEvent::Message {peer,message} => match message {

                                Message::Request { request, channel, .. } => {
                                    let req = String::from_utf8(request.0.clone()).unwrap();
                                    // -------- GET_PEERS --------
                                    if req == "GET_PEERS" {
                                        let peers = state.lock().unwrap().peers.clone();
                                        let list = peers.iter()
                                            .map(|(peer, _)| peer.to_string())
                                            .collect::<Vec<_>>()
                                            .join(",");
                                        let response = format!("PEERS|{}",list);
                                        let response_string = RvcResponse(response.into_bytes());
                                        swarm.behaviour_mut()
                                            .req_res
                                            .send_response(channel, response_string)
                                            .unwrap();
                                    }

                                    // -------- GET_REFS --------
                                    else if req == "GET_REFS" {
                                        let refs = get_refs();

                                        // format: "main abc123\ndev def456"
                                        let list = refs.iter()
                                            .map(|(k, v)| format!("{} {}", k, v))
                                            .collect::<Vec<_>>()
                                            .join("\n");
                                        let response_string = format!("REFS|{}",list);
                                        let response = RvcResponse(response_string.into_bytes());

                                        swarm.behaviour_mut()
                                            .req_res
                                            .send_response(channel, response)
                                            .unwrap();
                                    }
                                }

                                // response received
                                Message::Response { response, .. } => {

                                    let data = String::from_utf8(response.0).unwrap();

                                    // -------- REFS --------
                                    if data.starts_with("REFS|") {
                                        let payload = data.trim_start_matches("REFS|");

                                        let mut refs_map = std::collections::HashMap::new();

                                        for line in payload.lines() {
                                            let parts: Vec<_> = line.split_whitespace().collect();
                                            if parts.len() == 2 {
                                                refs_map.insert(parts[0].to_string(), parts[1].to_string());
                                            }
                                        }

                                        let mut st = state.lock().unwrap();
                                        st.peer_refs.insert(peer, refs_map.clone());

                                        println!("\nBranches from {}:", peer);
                                        for (branch, hash) in refs_map {
                                            println!("{} → {}", branch, hash);
                                        }
                                    }

                                    // -------- PEERS --------
                                    else if data.starts_with("PEERS|") {
                                        let payload = data.trim_start_matches("PEERS|");

                                        let mut st = state.lock().unwrap();

                                        for p in payload.split(",") {
                                            if let Ok(peer_id) = p.parse() {
                                                st.peers.entry(peer_id).or_insert_with(Vec::new);
                                            }
                                        }

                                        println!("\nAll Known Peers:");
                                        for (peer, addrs) in st.peers.iter() {
                                            println!("{} -> {:?}", peer, addrs);
                                        }
                                    }

                                    else {
                                        println!("Unknown response: {}", data);
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
