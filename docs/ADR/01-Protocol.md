# ADR01 - Wire Protocol

## Problem Statement
Problem building a high-performance cache to rival Hazelcast or GemFire, "wire" protocol and serialisation format is one of the most critical early design decisions. 

Two biggest concerns are:
* **Framing & versioning** over a raw TCP stream
* **Wire format** for messages.


## 1. TCP Framing & Versioning
Every message on the wire should be length-prefixed frame with fixed-size header

| Byte Offset | Length (bytes)    | Field           | Description                               |
| ----------- | ----------------- | --------------- | ----------------------------------------- |
| 0           | 4                 | FrameLength     | Total bytes following this field (u32 BE) |
| 4           | 1                 | ProtocolVersion | Major version (u8)                        |
| 5           | 1                 | MessageType     | u8 discriminator for message payload      |
| 6           | 2                 | Flags           | bit-flags (e.g. compression, tracing)     |
| 8           | (FrameLength − 4) | Payload         | Serialized body (see below)               |

> Note:
> *  **FrameLength** lets you read exactly that many bytes from socket, (don't over-read)
> * **ProtocolVersion** makes rolling incompatible changes safe - clients & servers can reject unsupported versions.
> * **MessageType** is use in demultiplexing different frame-level message. (Heartbeats, Version-negotiation or handshake frames, Control messages)
> * **Flags** are reserved for optional features (eg payload is gzip, includes a trace-id)


## 2. Wire Format: Custom vs Protobuf vs FlatBuffers
Some of the considerations made

| Criteria                | Custom Binary          | Protobuf                  | FlatBuffers            |
| ----------------------- | ---------------------- | ------------------------- | ---------------------- |
| **Schema evolution**    | You build support      | First-class, versioned    | First-class, versioned |
| **Language support**    | You write serializers  | > 20 languages            | \~10 languages         |
| **Zero-copy reads**     | Yes (if you design it) | No (parsing allocs)       | Yes                    |
| **Ease of use**         | High effort            | Very low effort           | Medium effort          |
| **Binary size**         | Tunable                | Compact, but not minimal  | Very compact           |
| **Tooling & ecosystem** | None beyond your code  | Built-in linting, plugins | Growing, but smaller   |


### Recommendation
Start with **Protocol Buffers**
* Pros:
  * Easy to define & generate code for Rust, Java, Python, Go.
  * Built-in versioning rules (adding/removing fields safely).
  * Well-undestood performance characteristics (you will pay a small parse cost).
* To consider **FlatBuffers**:
  * when we have scaffold and structured the project.
  * If zero-copy reads are critical (i.e. Need to handle millions of ops/sec and can’t afford any heap allocations).
  * If payloads are large, and require random-access into nested structures without unpacking.

### Summary

* Framing: 4-byte length + 1-byte version + 1-byte type + 2-byte flags.
* Wire format: Protobuf for day-1 (easy cross-language, schema evolution).
* .proto: Define a small Envelope/EnvelopeResponse with all your request/response types.
* Parsing: Use generated code, keep your Rust server zero-copy for the header, then prost::Message::decode for the rest.