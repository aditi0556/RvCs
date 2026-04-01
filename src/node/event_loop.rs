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
use std::io::Read;
use std::collections::HashSet;

//to find the common commit
pub fn find_common_commit(local: Vec<String>,remote: Vec<String>,) -> Option<String> {
    let remote_set: HashSet<String> = remote.into_iter().collect();
    for commit in local {
        if remote_set.contains(&commit) {
            return Some(commit);
        }
    }
    None
}

fn collect_all_objects(hash: &str, out: &mut Vec<String>) {
    if out.contains(&hash.to_string()) {
        return;
    }
    out.push(hash.to_string());

    let path = format!(".rvc/objects/{}/{}", &hash[..2], &hash[2..]);
    let Ok(raw) = std::fs::read(&path) else { return };

    // decompress (assuming zlib like git)
    let Ok(decompressed) = decompress_zlib(&raw) else { return };

    // object header format: "<type> <size>\0<content>"
    let Some(null_pos) = decompressed.iter().position(|&b| b == 0) else { return };
    let header = &decompressed[..null_pos];
    let content = &decompressed[null_pos + 1..];

    let Ok(header_str) = std::str::from_utf8(header) else { return };

    if header_str.starts_with("commit") {
        // commit content is text: find "tree <hash>\n"
        let Ok(text) = std::str::from_utf8(content) else { return };
        for line in text.lines() {
            if line.starts_with("tree ") {
                let tree_hash = line[5..].trim();
                collect_all_objects(tree_hash, out);
            }
            else if line.starts_with("parent ") {
                let parent_hash = line[7..].trim();
                collect_all_objects(parent_hash, out);
            }
            // stop at blank line (start of commit message)
            if line.is_empty() { break; }
        }
    } else if header_str.starts_with("tree") {
        // binary format: "<mode> <filename>\0<20 raw bytes>"
        let mut cursor = std::io::Cursor::new(content);
        loop {
            let mut mode = Vec::new();
            let mut filename = Vec::new();
            let mut raw_hash = vec![0u8; 20];

            use std::io::BufRead;
            if cursor.read_until(b' ', &mut mode).unwrap_or(0) == 0 { break; }
            if cursor.read_until(b'\0', &mut filename).unwrap_or(0) == 0 { break; }
            if cursor.read_exact(&mut raw_hash).is_err() { break; }

            // convert 20 raw bytes → 40 char hex string
            let entry_hash = raw_hash.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>();

            mode.pop(); // remove trailing space
            let Ok(mode_str) = std::str::from_utf8(&mode) else { break };

            match mode_str {
                "40000" => {
                    // subtree — recurse
                    collect_all_objects(&entry_hash, out);
                }
                "100644" | "100755" | "120000" => {
                    // blob — just add the hash, no need to recurse
                    if !out.contains(&entry_hash) {
                        out.push(entry_hash);
                    }
                }
                _ => {}
            }

            if cursor.position() as usize >= content.len() { break; }
        }
    }
    // blobs: no children to walk
}

pub fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    let mut decoder = ZlibDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

//here the missing commits is stored such that the first commit is the latest commit
pub fn get_missing_commits(commits: Vec<String>,base: &str,) -> Vec<String> {
    let mut result = Vec::new();
    for commit in commits {
        if commit == base {
            break;
        }
        result.push(commit);
    }
    result
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
                        let commits = crate::node::get_refs::get_all_commits_of_branch(&branch); 
                        let payload = format!("SYNC_REQ|{}|{}",branch,commits.join(","));
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
                                        let parts: Vec<&str> = req.split('|').collect();
                                        let branch = parts[1].to_string();
                                        let remote_commits: Vec<String> = parts[2]
                                            .split(',')
                                            .map(|s| s.to_string())
                                            .collect();
                                        let local_commits = crate::node::get_refs::get_all_commits_of_branch(&branch);
                                        let common = find_common_commit(local_commits.clone(), remote_commits);
                                        let missing = if let Some(base) = common {
                                            get_missing_commits(local_commits, &base)
                                        } else {
                                            local_commits
                                        };
                                        let response = format!("SYNC_RES|{}|{}",branch,missing.join(","));
                                        //send these commits to the requesting peer 
                                        swarm.behaviour_mut()
                                            .req_res
                                            .send_response(channel, RvcResponse(response.into_bytes()))
                                            .unwrap();
                                    }

                                    // -------- GET_OBJS --------
                                    else if req.starts_with("GET_OBJS|") {
                                        let parts: Vec<&str> = req.split('|').collect();
                                        if parts.len() < 3 {
                                            println!("Invalid GET_OBJS request");
                                        } else {
                                            let branch = parts[1];
                                            let requested: Vec<String> = parts[2]
                                                .split(',')
                                                .map(|s| s.to_string())
                                                .collect();

                                            // Expand each commit hash → tree → blobs recursively
                                            let mut all_hashes: Vec<String> = Vec::new();
                                            for hash in &requested {
                                                collect_all_objects(hash, &mut all_hashes);
                                            }
                                            all_hashes.dedup();

                                            let mut result = Vec::new();
                                            for hash in all_hashes {
                                                let path = format!(".rvc/objects/{}/{}", &hash[..2], &hash[2..]);
                                                match std::fs::read(&path) {
                                                    Ok(data) => {
                                                        let encoded = base64::encode(&data);
                                                        result.push(format!("{}:{}", hash, encoded));
                                                    }
                                                    Err(_) => println!("Missing object {}", hash),
                                                }
                                            }
                                            let response = format!("OBJS|{}|{}",branch, result.join(","));
                                            swarm.behaviour_mut()
                                                .req_res
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
                                        let parts: Vec<&str> = data.split('|').collect();
                                        let branch = parts[1].to_string();
                                        let missing: Vec<String> = if parts.len() > 2 && !parts[2].is_empty() {
                                            parts[2].split(',').map(|s| s.to_string()).collect()
                                        } else {
                                            vec![]
                                        };
                                        println!("Missing commits: {:?}", missing);
                                        if !missing.is_empty() {
                                            let req = format!("GET_OBJS|{}|{}",branch, missing.join(","));
                                            swarm.behaviour_mut().req_res.send_request(&peer,RvcRequest(req.into_bytes()));
                                        }
                                    }
                                    
                                    else if data.starts_with("OBJS|") {
                                        let payload = &data["OBJS|".len()..];
                                        let mut parts = payload.splitn(2, '|');
                                        let Some(branch) = parts.next() else { return };
                                        let Some(objects_str) = parts.next() else { return };
                                        let mut first_commit:Option<String>=None;
                                        // write every object to disk
                                        for entry in payload.split(',') {
                                            let mut iter = entry.splitn(2, ':');
                                            let (Some(hash), Some(b64)) = (iter.next(), iter.next()) else { continue };
                                            let Ok(bytes) = base64::decode(b64) else {
                                                println!("Failed to decode object {}", hash);
                                                continue;
                                            };
                                            if first_commit.is_none() {
                                               first_commit = Some(hash.to_string());
                                            }
                                            //now write it to the objects
                                            let dir  = format!(".rvc/objects/{}", &hash[..2]);
                                            let file = format!("{}/{}", dir, &hash[2..]);
                                            if !std::path::Path::new(&file).exists() {
                                                std::fs::create_dir_all(&dir).ok();
                                                std::fs::write(&file, &bytes).ok();
                                                println!("Wrote object {}", hash);
                                            }
                                        }

                                        if let Some(latest) = first_commit {
                                            // directly update .rvc/refs/remotes/<peer>/<branch>
                                            let remote_dir = format!(".rvc/refs/remotes/{peer}", peer = "peer_placeholder"); 
                                            std::fs::create_dir_all(&remote_dir).ok();
                                            let ref_file = format!("{}/{}", remote_dir, branch);
                                            std::fs::write(&ref_file, latest.as_bytes()).ok();
                                            println!("Updated remote branch {} → {}", branch, latest);
                                        }

                                        // now trigger merge using the stored pending_fetch
                                        // let pending = {
                                        //     let mut st = state.lock().unwrap();
                                        //     st.pending_fetch.take()  // take clears it from state
                                        // };

                                        // if let Some(fetch) = pending {
                                        //     let conflicts = merge_branch(&fetch.branch, &fetch.remote_head);
                                        //     if conflicts.is_empty() {
                                        //         println!("Merge complete.");
                                        //     } else {
                                        //         println!("Merge has {} conflict(s) — resolve manually:", conflicts.len());
                                        //         for f in conflicts {
                                        //             println!("  CONFLICT: {}", f);
                                        //         }
                                        //     }
                                        // }
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
