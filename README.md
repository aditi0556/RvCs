# RVC - Peer-to-Peer Git-like Version Control System

A distributed version control system built using Rust + libp2p, enabling Git-like operations with direct peer-to-peer syncing (no central server).

---

## Features

* Git-like object model (blob, tree, commit)
* Branching and switching
* Commit history tracking
* Peer-to-peer sync using libp2p
* Automatic peer discovery via mDNS
* Fetch and merge across peers
* Custom lightweight protocol (no Git server required)

---

## Installation

### 1. Build the project

```bash
cargo build --release
```

### 2. Copy binary into your repository

```bash
cp target/release/rvc /path/to/your/project
cd /path/to/your/project
```

### 3. Initialize repository

```bash
./rvc init
```

---

## CLI Commands

### Basic Commands

```bash
./rvc init
./rvc add <file>
./rvc commit -m "message"
./rvc hash-object <file>
./rvc cat-file <hash>
./rvc write-tree
./rvc ls-tree <tree_hash>
```

---

### Branching

```bash
./rvc create-branch <name>
./rvc switch-branch <name>
./rvc current-branch
```

---

### Clone

```bash
./rvc clone <repo_path>
```

---

### Start a Peer Node

```bash
./rvc join -p <port>
```

What happens:

* Starts a libp2p node
* Runs event loop in background
* Opens interactive CLI
* Enables automatic peer discovery (mDNS)

---

### Runtime Commands (after join)

```bash
rvcd discover
rvcd join <peer_id> <multiaddr>
rvcd branches <peer_id>
rvcd merge <peer_id> <branch>
```

---

## Peer Discovery (mDNS)

* Uses Multicast DNS
* Automatically discovers peers in local network

Shares:

* Peer ID
* Multiaddress

---

## Protocol Design

RVC uses a custom request-response protocol.

### GET_PEERS

```
GET_PEERS
```

Response:

```
PEERS|peer1,peer2,peer3
```

---

### GET_REFS

```
GET_REFS
```

Response:

```
REFS|main abc123
dev def456
```

---

### SYNC_REQ

```
SYNC_REQ|<branch>|<local_commits>
```

---

### SYNC_RES

```
SYNC_RES|<branch>|<missing_commits>
```

---

### GET_OBJS

```
GET_OBJS|<branch>|<commit_hashes>
```

Response:

```
OBJS|<branch>|hash1:base64,hash2:base64
```

---

## Sync Flow

```
Peer A -> SYNC_REQ -> Peer B
Peer B -> SYNC_RES -> Peer A
Peer A -> GET_OBJS -> Peer B
Peer B -> OBJS -> Peer A
Peer A writes objects -> merge triggered
```

---

## Event Loop Architecture

```
CLI Input
    ->
Command Channel
    ->
Event Loop (tokio)
    -> Commands (send request)
    -> Network (handle events, mDNS)
```

---

## Internal Architecture

* CLI Layer (clap) - parses commands
* Command Layer - Git-like operations
* Node Layer - libp2p networking
* Event Loop - async orchestration
* State (Arc<Mutex>) - shared memory

---

## Shared State

```rust
Arc<Mutex<AppState>>
```

Stores:

* peers
* peer_refs
* pending_fetch

---

## Peer Model

* Each node connects to multiple peers
* Identified via:

  * PeerId
  * Multiaddr
* Communication via request-response channels

---

## Future Improvements

* Packfile support
* Internet-wide peer discovery (beyond mDNS)
* Load balancing
* Improved conflict resolution

---

## Summary

RVC is:

* Distributed Git-like system
* Fully peer-to-peer
* No central server
* Built with Rust + libp2p
* Supports real repository syncing

---
