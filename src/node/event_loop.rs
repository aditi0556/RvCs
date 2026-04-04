use crate::node::{
    behaviour::{RvcBehaviour, RvcBehaviourEvent},
    command::Command,
    get_refs::get_refs,
    protocol::{RvcRequest, RvcResponse},
    state::AppState,
    objects::collect_all_objects,
    merge,
};
use futures::StreamExt;
use libp2p::{
    mdns,
    request_response::{Event as ReqResEvent, Message},
    swarm::{Swarm, SwarmEvent},
};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::Receiver;
use crate::command::refs;

pub fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    let mut decoder = ZlibDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

//here the missing commits is stored such that the first commit is the latest commit
pub fn get_missing_commits(local_commits: &[String], lca: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut found = false;
    for commit in local_commits {
        if commit == lca {
            found = true;
            break;
        }
        result.push(commit.clone());
    }
    if !found {
        println!("Warning: LCA {} not found in remote commit list", &lca[..7]);
    }
    result // newest-first, so result[0] is B's HEAD
}

pub async fn create_event_loop(mut swarm: Swarm<RvcBehaviour>,state: Arc<Mutex<AppState>>,mut rx: Receiver<Command>,) {
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
                    Command::Merge{ peer, branch } => {
                        println!("Merging branch {} with {}", branch, peer);
                        let local_branch = refs::get_current_branch().unwrap_or("detached".to_string());
                        println!("Current branch here is {}",local_branch);
                        let local_commits = crate::node::get_refs::get_all_commits_of_branch(&local_branch);
                        let payload = format!("SYNC_REQ|{}|{}",branch,local_commits.join(","));
                        {
                            let mut st = state.lock().unwrap();
                            st.pending_fetch = Some(crate::node::state::PendingFetch{
                                                 branch:local_branch.clone(),
                                                 remote_head: String::new(),
                                                });
                        }
                        swarm.behaviour_mut().req_res.send_request(&peer,RvcRequest(payload.into_bytes()));
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

                    SwarmEvent::ConnectionClosed { peer_id, cause, .. } => {
                        println!("Disconnected from {} cause: {:?}", peer_id, cause);
                    }

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

                                    // -------- SYNC_REQ --------
                                    else if req.starts_with("SYNC_REQ|") {
                                        println!("Got the request, SYNC_REQ");
                                        
                                        let parts: Vec<&str> = req.splitn(3, '|').collect();  // ← FIX: splitn(3) not split
                                        println!("Parts count: {}", parts.len());
                                        
                                        if parts.len() < 3 {
                                            println!("ERROR: SYNC_REQ malformed, only {} parts", parts.len());
                                            return;
                                        }
                                        
                                        let branch = parts[1].to_string();
                                        println!("Branch name is {}", branch);
                                        
                                        let remote_commits: Vec<String> = parts[2]
                                            .split(',')
                                            .filter(|s| !s.is_empty())
                                            .map(|s| s.to_string())
                                            .collect();
                                        println!("Received {} remote commits from peer A", remote_commits.len());
                                        
                                        let head = crate::node::get_refs::read_ref(&branch);
                                        println!("Local HEAD for branch '{}': '{}'", branch, head);
                                        
                                        if head.is_empty() {
                                            println!("Branch '{}' not found locally — will send all commits", branch);
                                        }
                                        
                                        let common = if head.is_empty() {
                                            None
                                        } else {
                                            let lca = crate::node::get_refs::find_lca(head.as_str(), &remote_commits);
                                            println!("LCA result: {:?}", lca.as_deref().map(|h| &h[..7.min(h.len())]));
                                            lca
                                        };

                                        let local_commits = crate::node::get_refs::get_all_commits_of_branch(&branch);
                                        println!("Local commits count: {}", local_commits.len());

                                        let missing = if let Some(ref base) = common {
                                            println!("Common base found: {}", &base[..7.min(base.len())]);
                                            get_missing_commits(&local_commits, base)
                                        } else {
                                            println!("No common base — sending all {} local commits", local_commits.len());
                                            local_commits
                                        };
                                        
                                        println!("Sending {} missing commits in SYNC_RES", missing.len());
                                        
                                        let response = format!("SYNC_RES|{}|{}", branch, missing.join(","));
                                        println!("Response size: {} bytes", response.len());
                                        
                                        match swarm.behaviour_mut()
                                            .req_res
                                            .send_response(channel, RvcResponse(response.into_bytes()))
                                        {
                                            Ok(_)  => println!("SYNC_RES sent successfully"),
                                            Err(e) => println!("ERROR sending SYNC_RES: {:?}", e),
                                        }
                                    }
                                    // -------- GET_OBJS --------
                                     else if req.starts_with("GET_OBJS|") {
                                        println!("Got the GET_OBJS");
                                        let parts: Vec<&str> = req.splitn(3, '|').collect();
                                        if parts.len() < 3 {
                                            println!("Invalid GET_OBJS");
                                        } else {
                                            let branch = parts[1];
                                            let requested: Vec<String> = parts[2].split(',').map(|s| s.to_string()).collect();

                                            let mut all_hashes: Vec<String> = Vec::new();
                                            for hash in &requested {
                                                collect_all_objects(hash, &mut all_hashes);
                                            }
                                            all_hashes.dedup();

                                            let mut result = Vec::new();
                                            for hash in all_hashes {
                                                let path = format!(".git/objects/{}/{}", &hash[..2], &hash[2..]);
                                                match std::fs::read(&path) {
                                                    Ok(data) => result.push(format!("{}:{}", hash, base64::encode(&data))),
                                                    Err(_)   => println!("Missing object {}", hash),
                                                }
                                            }
                                            let response = format!("OBJS|{}|{}", branch, result.join(","));
                                            swarm.behaviour_mut().req_res
                                                .send_response(channel, RvcResponse(response.into_bytes()))
                                                .unwrap();
                                        }
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
                                    //-------SYNC_RES-----------
                                    else if data.starts_with("SYNC_RES|") {
                                        println!("Got SYNC_RES");
                                        let parts: Vec<&str> = data.split('|').collect();
                                        // let branch = parts[1].to_string();
                                        let missing: Vec<String> = if parts.len() > 2 && !parts[2].is_empty() {
                                            parts[2].split(',').map(|s| s.to_string()).collect()
                                        } else {
                                            vec![]
                                        };

                                        //this gives the HEAD commit of the remote peer
                                        println!("Missing commits: {:?}", missing);
                                        if !missing.is_empty() {
                                            let remote_head = missing[0].clone();
                                            {
                                                let mut st = state.lock().unwrap();
                                                if let Some(ref mut fetch) = st.pending_fetch{
                                                    fetch.remote_head = remote_head.clone();
                                                }
                                            }
                                            let branch = {
                                                state.lock().unwrap()
                                                    .pending_fetch
                                                    .as_ref()
                                                    .map(|f| f.branch.clone())
                                                    .unwrap_or_default()
                                            };
                                            let req = format!("GET_OBJS|{}|{}",branch, missing.join(","));
                                            swarm.behaviour_mut().req_res.send_request(&peer,RvcRequest(req.into_bytes()));
                                        }

                                    }
                                    
                                    // ── OBJS | branch | hash:b64,hash:b64,... ──
                                    // Peer receives objects, writes to disk, triggers merge
                                    else if data.starts_with("OBJS|") {
                                        println!("Got the OBJS repsosne");
                                        let parts: Vec<&str> = data.splitn(3, '|').collect();
                                        if parts.len() < 3 { return; }
                                        let objects_str = parts[2];

                                        // write all objects to disk first
                                        for entry in objects_str.split(',') {
                                            let mut iter = entry.splitn(2, ':');
                                            let (Some(hash), Some(b64)) = (iter.next(), iter.next()) else { continue };
                                            let Ok(bytes) = base64::decode(b64) else { continue };
                                            let dir  = format!(".git/objects/{}", &hash[..2]);
                                            let file = format!("{}/{}", dir, &hash[2..]);
                                            if !std::path::Path::new(&file).exists() {
                                                std::fs::create_dir_all(&dir).ok();
                                                std::fs::write(&file, &bytes).ok();
                                                println!("Wrote object {}", hash);
                                            }
                                        }

                                        // retrieve pending fetch info and merge
                                        let pending = state.lock().unwrap().pending_fetch.take();
                                        if let Some(fetch) = pending {
                                            println!("Merging now!!!");
                                            merge::merge_branch(&fetch.branch, &fetch.remote_head);
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
