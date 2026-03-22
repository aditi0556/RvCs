use libp2p::{
    mdns,
    request_response,
    request_response::Behaviour as RequestResponse,
    swarm::NetworkBehaviour,
};

#[derive(NetworkBehaviour)]
#[behaviour(to_swarm = "RvcBehaviourEvent")]
pub struct RvcBehaviour{
    pub mdns: mdns::tokio::Behaviour,
    pub req_res: RequestResponse<crate::node::protocol::RvcCodec>,
}
#[derive(Debug)]
pub enum RvcBehaviourEvent {
    Mdns(mdns::Event),
    ReqRes(request_response::Event<
        crate::node::protocol::RvcRequest,
        crate::node::protocol::RvcResponse,
    >),
}

impl From<mdns::Event> for RvcBehaviourEvent {
    fn from(event: mdns::Event) -> Self {
        RvcBehaviourEvent::Mdns(event)
    }
}

impl From<
    request_response::Event<
        crate::node::protocol::RvcRequest,
        crate::node::protocol::RvcResponse,
    >,
> for RvcBehaviourEvent {
    fn from(event: request_response::Event<
        crate::node::protocol::RvcRequest,
        crate::node::protocol::RvcResponse,
    >) -> Self {
        RvcBehaviourEvent::ReqRes(event)
    }
}