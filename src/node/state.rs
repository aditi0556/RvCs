use std::collections::{HashMap,HashSet};
use libp2p::{PeerId,Multiaddr};

pub struct PendingFetch{
    pub branch:String,
    pub remote_head:String,
}
#[derive(Default)]
pub struct AppState {
    pub peers: HashMap<PeerId, Vec<Multiaddr>>,
    pub peer_refs: HashMap<PeerId, HashMap<String, String>>,
    pub pending_fetch : Option<PendingFetch>,
    pub connected_peers: HashSet<PeerId>, 
}