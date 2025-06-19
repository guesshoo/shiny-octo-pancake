# Comparisons: Hazelcast (Enterprise, OpenSource), GemFire, and Potential Custom Cache.

Understand and compare existing alternatives (**Hazelcast (open source and enterprise)** and **VMware GemFire**), prior to building new cache product-specifically one with *ZeroCopy* capabilities.

---

## 🔹 Hazelcast (Open Source)

### ✅ Key Features

* In-Memory Data Grid (IMDG)
* Distributed collections (`IMap`, `IQueue`, etc.)
* Pub/Sub messaging via topics
* SQL over data structures
* Entry processors for server-side operations
* Near Cache and basic persistence

🔗 [Hazelcast Open Source Docs](https://docs.hazelcast.com/hazelcast/latest)

---

## 🔷 Hazelcast Enterprise

### ✅ Enterprise-Only Features

* **High-Density Memory Store (HD Memory):** off-heap, GC-free storage
  🔗 [HD Memory (Hazelcast Enterprise)](https://docs.hazelcast.com/hazelcast/5.5/storage/high-density-memory#hide-nav)
* **Hot Restart Store:** fast node recovery
  🔗 [Hot Restart Store](https://hazelcast.com/products/hot-restart-store/)
* **WAN Replication (Advanced):** multi-datacenter with conflict resolution
  🔗 [WAN Replication](https://docs.hazelcast.com/management-center/5.8/clusters/wan-replication#hide-nav)
* **CP Subsystem:** consensus-based operations (Raft)
  🔗 [CP Subsystem](https://docs.hazelcast.com/hazelcast/5.5/cp-subsystem/management#hide-nav)
* **Security features**: TLS, LDAP, JAAS
  🔗 [Security (Enterprise)](https://docs.hazelcast.com/hazelcast/5.5/security/overview#hide-nav)
* **Rolling Upgrades and Management Center**
  🔗 [Management Center](https://docs.hazelcast.com/management-center/5.8/getting-started/overview#hide-nav)

---

## 🟥 VMware GemFire

### ✅ Key Features

* Partitioned and replicated regions
* Continuous Query (CQ)
  🔗 [Continuous Query (GemFire)](https://docs.vmware.com/en/VMware-GemFire/9.15/gf-developing/continuous-query.html)
* Durable clients, async queues
* Disk persistence (WAL, snapshotting)
* Full support for transactions
  🔗 [Transactions in GemFire](https://docs.vmware.com/en/VMware-GemFire/9.15/gf-developing/transactions.html)
* Function execution (server-side compute)
* WAN replication and gateways
  🔗 [WAN Gateway](https://docs.vmware.com/en/VMware-GemFire/9.15/gf-admin/wide-area-networking.html)
* PDX Serialization Format
  🔗 [PDX Serialization](https://docs.vmware.com/en/VMware-GemFire/9.15/gf-developing/serialization/serializing-data-with-pdx.html)

🔗 [GemFire Documentation Home](https://docs.vmware.com/en/VMware-GemFire/index.html)

---

## 📦 High-Level Data Structures Comparison

| Feature / Capability   | Hazelcast OSS                       | Hazelcast Enterprise                    | VMware GemFire                   |
|------------------------|-------------------------------------|-----------------------------------------|----------------------------------|
| Data Structures        | IMap, ISet, IQueue, IList, MultiMap | Same + Off-heap maps                    | Regions (Partitioned/Replicated) |
| SQL Support            | Basic SQL for maps                  | Full SQL engine                         | OQL (Object Query Language)      |
| Off-Heap / ZeroCopy    | ❌                                 | HD Memory (off-heap, not true ZeroCopy) | PDX (optimized, not ZeroCopy)    |
| Persistence & WAL      | Basic MapStore                      | Hot Restart, Snapshots                  | Full WAL & Disk Stores           |
| Transactions           | No                                  | Optional CP Subsystem                   | Full ACID support                |
| WAN Replication        | Basic                               | Advanced                                | Yes                              |
| Custom Data Format     | Java Serialization or Portable      | Custom serializers                      | PDX Format                       |
| Multi-language Clients | Java, limited REST                  | Java, SQL, TLS                          | Java, C++, .NET                  |


