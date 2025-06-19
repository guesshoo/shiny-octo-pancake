# 💰 Total Cost of Owenership (TCO)

* A detailed **TCO (Total Cost of Ownership) model comparison** between:
    * **Hazelcast Open Source**
    * **Hazelcast Enterprise**
    * **GemFire / Apache Geode**
    * Propose **Custom ZeroCopy Cache (LMDB + FlatBuffers)**
  
* Looking at **3-year cost projection** across several key dimensions to support long-term strategic planning.

## Components of TCO
* **Licensing Cost**s – Upfront and subscription fees (e.g., Hazelcast Enterprise, GemFire).
* **Hardware / Infrastructure** – Servers, cloud instances, storage, networking.
* **Development & Integration** – Engineering time to build, customize, or integrate.
* **Operations & Maintenance** – On-call burden, monitoring, scaling, bug fixing.
* **Training & Documentation** – Time to onboard teams and maintain internal knowledge.
* **Vendor Lock-in Risks** – Switching costs and constraints with commercial platforms.
 

---

## 📊 TCO Comparison (Estimates)

| **Cost Dimension**        | Hazelcast OSS         | Hazelcast Enterprise                | GemFire / Geode                | Custom ZeroCopy Cache             |
|---------------------------|-----------------------|-------------------------------------|--------------------------------|-----------------------------------|
| **Licensing**             | $0                    | ~$22k/year/node                     | $2.4K/year/core                | $0                                |
| **Infra (Cloud/On-Prem)** | Moderate              | High (needs more memory/GC tuning)  | High (heavier JVM footprint)   | Low (tight resource control)      |
| **Engineering (Dev)**     | Moderate/High         | High (integration, tuning)          | High (complex API)             | Medium (initial build)            |
| **Engineering (Ops)**     | Moderate              | High (monitoring, tuning, upgrades) | High                           | Low (simple binary, snapshotting) |
| **Training / Support**    | Low                   | High (docs, vendor support req.)    | High                           | Low (minimal surface)             |
| **Custom Feature Cost**   | High (hard to extend) | Moderate (plugin support)           | Very High (rigid, enterprisey) | Low (custom from day 1)           |
| **Vendor Lock-In Risk**   | Medium                | High                                | High                           | None                              |

---

## 📉 Estimated TCO Breakdown (for a 5-node production cluster)

### Assumptions:

* 5-node cluster
* 3-year planning horizon
* Mid-sized node (e.g., m5.4xlarge)	16 cores
  
| **Cost Dimension**     | Hazelcast OSS | Hazelcast Enterprise | GemFire (Tanzu)      | Custom ZeroCopy Cache  |
|------------------------|---------------|----------------------|----------------------|------------------------|
| **Licensing**          | $0            | ~$340,000 (5 nodes)  | ~$580,000 (80 cores) | $0                     |
| **Infrastructure**     | ~$180,000     | ~$220,000            | ~$250,000            | ~$120,000              |
| **Engineering (Dev)**  | ~$100,000     | ~$100,000            | ~$120,000            | ~$80,000 (initial dev) |
| **Operations (Ops)**   | ~$90,000      | ~$150,000            | ~$160,000            | ~$50,000               |
| **Training & Support** | ~$15,000      | ~$45,000             | ~$60,000             | ~$10,000               |
| **Total (3-Year)**     | **$385,000**  | **$855,000**         | **$1,170,000**       | **$260,000**           |

  * *These examples are illustrative; actual pricing depends on negotiation, core counts, nodes, support tiers, and multi-year discounts.*
---

## ✅ Key Points

* **Hazelcast OSS** is inexpensive upfront but requires **moderate engineering effort** and lacks off-heap/ZeroCopy support.
* **Hazelcast Enterprise** and **GemFire** have steep TCO due to **licensing, tuning, and complexity overhead**.
*  A **Custom Cache** has **higher upfront dev costs**, but **very low long-term TCO**, excellent performance, and **no lock-in**.




## ℹ️ References and Further Details

### Hazelcast Enterprise Licensing
* Specific pricing details are not published openly - must contact their sales team.
* However third-party market place data suggests typical per-node Enterprise costs range between **$12,764 and $36,828/year**, with **a median of $22,782/year**. [vendr](https://www.vendr.com/marketplace/hazelcast)

### VMware Tanzu GemFire Licensing
* Pricing from the [UK G‑Cloud contract in 2022 ](https://assets.applytosupply.digitalmarketplace.service.gov.uk/g-cloud-13/documents/92295/171284320967119-pricing-document-2022-05-18-1048.pdf) indicates per-core premium support subscription costs:
  * £1,788.66/year per core for GemFire (minimum 9 cores)
  * £5,365.98 for a 3‑year subscription per core.


| Product                  | Price Basis              | Example 3-Year Cost*                            |
|--------------------------|--------------------------|-------------------------------------------------|
| **Hazelcast Enterprise** | Median $22,782/node/year | 5 nodes → $341K                                 |
| **Tanzu GemFire**        | £$5,365.98/core/3-year   | 80 cores → £48,300 (~$575K using 0.74 USD/GBP ) |


### Typical Relationship between cores per node
| Node Type                                 | Typical vCPU/Core Count |
|-------------------------------------------|-------------------------|
| Small cloud instance (e.g., t3.medium)    | 2 cores                 |
| Mid-sized node (e.g., m5.4xlarge)         | 16 cores                |
| High-performance node (e.g., c6i.8xlarge) | 32–64 cores             |
