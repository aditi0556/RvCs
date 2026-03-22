// use simple_mdns::sync_discovery::{ServiceDiscovery, OneShotMdnsResolver};
// use simple_mdns::InstanceInformation;
// use std::{net::SocketAddr, thread, time::Duration};

// const SERVICE_NAME: &str = "_p2p._tcp.local";

// pub fn run_server(name: String, port: u16) {
//     // Bind to all interfaces so mDNS works on localhost
//     let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().unwrap();
//     println!("🚀 Starting peer: {} at {}", name, addr);

//     let instance = InstanceInformation::new(name.clone())
//         .with_socket_address(addr);

//     let _discovery = ServiceDiscovery::new(
//         instance,
//         SERVICE_NAME,
//         60, // TTL in seconds
//     )
//     .expect("Failed to start mDNS");

//     println!("📡 Advertising on mDNS...");

//     // Keep the peer alive
//     loop {
//         thread::sleep(Duration::from_secs(5));
//     }
// }

// pub fn discover_peers() {
//     println!("🔍 Discovering peers...");
//     let resolver = OneShotMdnsResolver::new().expect("Failed to create resolver");

//     loop {
//         match resolver.query_service_address_and_port(SERVICE_NAME) {
//             Ok(services) => {
//                 println!("------------------------");
//                 for s in services {
//                     println!("Peer: {:?}", s);
//                 }
//             }
//             Err(e) => println!("Error: {:?}", e),
//         }

//         thread::sleep(Duration::from_secs(5));
//     }
// }