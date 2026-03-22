use libp2p::{PeerId,Multiaddr};

pub enum Command {
    Discover, 
    Dial {
        peer:PeerId,addr:Multiaddr,
    }
    // SendMessage { peer: PeerId, data: Vec<u8> },
}