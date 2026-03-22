use std::collections::HashMap;
use libp2p::{PeerId,Multiaddr};

#[derive(Default)]
pub struct AppState {
    pub peers: HashMap<PeerId, Vec<Multiaddr>>,
}