use libp2p::{Multiaddr, PeerId};

pub enum Command {
    Discover,
    Dial { peer: PeerId, addr: Multiaddr },
    Branches { peer: PeerId }, // SendMessage { peer: PeerId, data: Vec<u8> },
}
