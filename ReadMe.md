# Overview
Building a high-performance cache to rival HazelCast or GemFire

## Dev Setup
* Rust (1.84) - server
* `Protoc`: Protobuf Compiler
* 


## Development Phase

### Phase 1: Protocol Design, Storage and WAL
1. Prototype wire protocol.
2. Memory-mapped storage.
3. Write-Ahead Log (WAL) - Purpose:

   * Crash Recovery: before committing to LMDB, append each write operation to a sequential log file on disk.

    * Replication: ship/stream the same log records to follower nodes for multi-node synchronization. 

    * Format: append-only, record-oriented (e.g. [timestamp][term][operation][key][value]).

    * Rotation & Compaction: after LMDB checkpoint (or once WAL grows beyond threshold), snapshot current state and truncate/rotate old log segments.
 

### Phase 2: TCP Server Setup
