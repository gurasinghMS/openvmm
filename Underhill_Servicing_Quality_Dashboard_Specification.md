# Underhill Servicing Quality Dashboard - Comprehensive Specification

**Date:** April 2, 2026  
**Data Source:** `wdgeventstore.kusto.windows.net` → `CCA` → `UnderhillTestServicingQualityMV`  
**Purpose:** Monitor Underhill/OpenHCL/OpenVMM servicing quality with focus on blackout times and operational health

---

## Executive Summary

### Data Profile (Last 15 Days)
- **Total Events:** 3.8M successful servicing operations
- **Success Rate:** 99.9999% (only 1 failure in dataset)
- **Time Range:** March 18 - April 2, 2026
- **Coverage:** 227 clusters, 2,018 nodes, 5,849 VMs (last 7 days)
- **Servicing Types:** 94% self-servicing, 6% firmware upgrades

### Key Performance Metrics
| Metric | P50 | P95 | P99 | Max |
|--------|-----|-----|-----|-----|
| **Host Blackout** | 244ms | 674ms | 888ms | 20,153ms |
| **Guest Blackout** | 245ms | 675ms | 890ms | - |
| **Kernel Boot Time** | 146ms | 246ms | 300ms | 1,841ms |
| **Logs Flush Time** | 168ms | 369ms | - | 501,162ms |

### Critical Findings

**✅ Excellent Overall Reliability**
- 99.9999% success rate across 3.8M operations
- Consistent performance across most clusters and SKUs

**🚨 MAJOR VERSION PERFORMANCE GAP**
- **Version 1.7 is 2.8× faster than 1.6** for self-servicing (P50: 237ms vs 653ms, P95: 653ms vs 964ms)
- **1.6 self-servicing is the worst-performing operation** across all patterns
- **104K events still on 1.6** (7% of traffic in last 7 days) - migration to 1.7 should be accelerated
- **1.6 → 1.7 upgrades achieve best P95: 482ms** - better than even 1.7 self-servicing

**💻 CPU SIZE MATTERS - But Not How You'd Expect**
- **Small VMs (2-4 CPUs) benefit MOST from 1.7 migration:** 2.5-3.0× improvement
  - 4 CPU VMs: 819ms → 268ms (P95) = **551ms saved per event!**
  - 2 CPU VMs: 783ms → 312ms (P95) = **471ms saved per event**
- **Large VMs (32-96 CPUs) on 1.6 have WORST absolute performance:**
  - 96 CPU + 1.6: P95 = 1164ms (>1 second blackout!)
  - 64 CPU + 1.6: P95 = 998ms
  - 32 CPU + 1.6: P95 = 995ms
- **CPU count doesn't linearly correlate with worse performance** - version matters more
- **23.7% of 32-CPU VMs still on 1.6** (highest % - urgent migration target)

**⚠️ Performance Variations**
- **VM Generation 10.3** has 2x higher blackout times (P95: 934ms vs 416ms for Gen 9.1)
- **Ingrasys-Azure-Compute-GP-MM-CR-ARM-WCS-C2141** SKU shows occasional outliers (up to 11.9s)
- **Firmware downgrades** (1.7→1.6) show higher blackout (P95: 768ms vs 477ms for upgrades)

**🎯 Monitoring Priorities**
1. **Track major version adoption (1.6 vs 1.7)** - accelerate migration to 1.7
2. **Monitor CPU-specific performance** - prioritize 2-4 CPU and 32-96 CPU migrations
3. **Track transition type performance** - ensure upgrades remain fast
4. Monitor VM Generation 10.3 performance (2x higher blackout)
5. Watch for cluster-level regressions
6. Identify outliers (>P99) for investigation

---

## Dashboard Visualizations

### **VIZ 1: Performance by Major Version and Transition Type** ⭐ CRITICAL

**Purpose:** Compare self-servicing performance across major versions (1.6 vs 1.7) and transition types  
**Chart Type:** Grouped bar chart  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** Transition type (categorical):
  - 1.6 Self-Servicing
  - 1.7 Self-Servicing
  - 1.6 → 1.7 Upgrade
  - 1.7 → 1.6 Downgrade
- **Y-axis:** Blackout time (milliseconds, 0-1200ms range)
- **Bar Groups (per transition):**
  - P50 (blue bar)
  - P95 (red bar, thicker)
  - P99 (dark red bar)
- **Annotations:**
  - Event count below each transition type
  - Green checkmark ✓ for best performer in each percentile
  - Red warning ⚠️ for worst performer
- **Color Coding:** P95 bar color = green if <500ms, yellow if 500-800ms, red if >800ms

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| extend OldMajorMinor = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| extend NewMajorMinor = iff(NewVmFirmwareIgvmVersion == "", "", extract(@"^(\d+\.\d+)", 1, NewVmFirmwareIgvmVersion))
| extend TransitionPath = case(
    NewVmFirmwareIgvmVersion == "", strcat(OldMajorMinor, " Self-Servicing"),
    NewMajorMinor == "", strcat(OldMajorMinor, " Self-Servicing"),
    OldMajorMinor == NewMajorMinor, strcat(OldMajorMinor, " Minor Update"),
    strcat(OldMajorMinor, " → ", NewMajorMinor)
)
| summarize 
    EventCount = count(),
    HostBlackout_P50 = percentile(HostBlackoutMS, 50),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95),
    HostBlackout_P99 = percentile(HostBlackoutMS, 99),
    GuestBlackout_P95 = percentile(GuestBlackoutMS, 95),
    KernelBoot_P95 = percentile(KernelBootTimeMS, 95)
    by TransitionPath
| order by EventCount desc
```

**Expected Output:** 4-5 transition types

**Insights from Data (Last 7 Days):**
| Transition Type | Count | P50 | P95 | P99 | Performance |
|-----------------|-------|-----|-----|-----|-------------|
| **1.7 Self-Servicing** | **1.31M** | **237ms** ✓ | **653ms** | 923ms | ✅ **BEST** (88% of traffic) |
| 1.6 Self-Servicing | 104K | 653ms ⚠️ | 964ms ⚠️ | 1175ms ⚠️ | 🚨 **2.8x WORSE P50** than 1.7 |
| **1.6 → 1.7 Upgrade** | 45.8K | 257ms | **482ms** ✓ | **670ms** ✓ | ✅ **BEST P95/P99** |
| 1.7 → 1.6 Downgrade | 40.9K | 354ms | 768ms | 814ms | ⚠️ 59% worse P95 than upgrade |

**🎯 KEY FINDINGS:**
1. **Version 1.7 is 2.8× faster than 1.6** for self-servicing (P50: 237ms vs 653ms)
2. **1.6 → 1.7 upgrades have the best overall performance** (P95: 482ms)
3. **1.6 self-servicing is the worst operation** - migrate all 1.6 VMs to 1.7 ASAP!
4. **Downgrades (1.7 → 1.6) are expensive** but still better than staying on 1.6

**Action Items:**
- Accelerate migration of all 1.6 VMs to 1.7 (104K events = 7% of traffic still on old version)
- Investigate why downgrades are slower than upgrades (different code path?)
- Set P95 target: <500ms for all operations (currently only 1.6→1.7 achieves this)

---

### **VIZ 2: Blackout Time Trends Over Time** ⭐ CRITICAL

**Purpose:** Real-time monitoring to detect sudden regressions or infrastructure issues  
**Chart Type:** Multi-line time series  
**Update Frequency:** Hourly

**Visual Design:**
- **X-axis:** Time (hourly bins, rolling 24-hour window)
- **Y-axis:** Blackout time (milliseconds)
- **Lines:**
  - Host P50 (solid blue, thin)
  - Host P95 (solid red, thick)
  - Host P99 (dashed red)
- **Alert Threshold:** Horizontal dashed line at 800ms (P95 warning threshold)
- **Color zones:** Green background <500ms, yellow 500-800ms, red >800ms

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(24h)
| where UnderhillSvcExecutionStatus == "succeeded"
| where NewVmFirmwareIgvmVersion == ""  // Self-servicing only
| summarize 
    EventCount = count(),
    HostBlackout_P50 = percentile(HostBlackoutMS, 50),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95),
    HostBlackout_P99 = percentile(HostBlackoutMS, 99)
    by bin(UnderhillSvcExecutionStartTime, 1h)
| order by UnderhillSvcExecutionStartTime asc
```

**Expected Output:** 24 data points (last 24 hours)

**Insights from Data:**
- Hourly P95 typically ranges 600-750ms
- Hour-to-hour variations of ±20% are normal
- Sustained spikes >800ms indicate infrastructure issues
- Low event counts (<5000/hour) may signal data pipeline delays

**Use Case:** Detect infrastructure problems, cluster outages, or sudden performance degradation independent of firmware versions.

---

### **VIZ 3: Success Rate Over Time** ⭐ CRITICAL

**Purpose:** Overall servicing health and failure detection  
**Chart Type:** Line chart with percentage scale  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** Time (daily bins, 30-day window)
- **Y-axis:** Success rate (percentage, 99.0% - 100.0% range)
- **Line:** Success rate (green when >99.9%, yellow when 99.5-99.9%, red when <99.5%)
- **Target Line:** 99.9% SLA threshold (horizontal dashed line)
- **Annotations:** Show absolute failure count on hover

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(30d)
| summarize 
    Total = count(),
    Succeeded = countif(UnderhillSvcExecutionStatus == "succeeded"),
    Failed = countif(UnderhillSvcExecutionStatus == "failed")
    by bin(UnderhillSvcExecutionStartTime, 1d)
| extend SuccessRate = (Succeeded * 100.0) / Total
| project UnderhillSvcExecutionStartTime, SuccessRate, Total, Failed
| order by UnderhillSvcExecutionStartTime asc
```

**Expected Output:** 30 data points (one per day)

**Insights from Data:**
- Current success rate is 99.9999%
- Only 1 failure observed in entire 15-day dataset
- Daily volume ranges 128K-334K events

---

### **VIZ 4: Blackout Distribution Histogram** ⭐ HIGH

**Purpose:** Understand distribution shape and identify outlier thresholds  
**Chart Type:** Dual histogram (overlaid or side-by-side)  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** Blackout time (milliseconds, 0-2000ms range, 50ms bins)
- **Y-axis:** Count of events (log scale recommended)
- **Bars:**
  - Host blackout (blue, semi-transparent)
  - Guest blackout (orange, semi-transparent)
- **Annotations:**
  - Mark P50, P95, P99 with vertical lines
  - Highlight outlier region (>P99) in red

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| extend HostBlackoutBin = bin(HostBlackoutMS, 50)  // 50ms bins
| extend GuestBlackoutBin = bin(GuestBlackoutMS, 50)
| summarize 
    HostCount = count() by HostBlackoutBin
| join kind=fullouter (
    UnderhillTestServicingQualityMV
    | where UnderhillSvcExecutionStartTime > ago(7d)
    | where UnderhillSvcExecutionStatus == "succeeded"
    | extend GuestBlackoutBin = bin(GuestBlackoutMS, 50)
    | summarize GuestCount = count() by GuestBlackoutBin
) on $left.HostBlackoutBin == $right.GuestBlackoutBin
| project 
    BlackoutBin = coalesce(HostBlackoutBin, GuestBlackoutBin),
    HostCount = coalesce(HostCount, 0),
    GuestCount = coalesce(GuestCount, 0)
| where BlackoutBin < 2000  // Focus on main distribution
| order by BlackoutBin asc
```

**Expected Output:** ~40 bins covering 0-2000ms range

**Insights from Data:**
- Bimodal distribution: fast path (200-250ms) and slow path (500-700ms)
- ~75% of events complete under 406ms (P75)
- Long tail extends to 20+ seconds (rare outliers)
- Host and guest blackout distributions are nearly identical

---

### **VIZ 5: Performance by VM Generation** ⭐ HIGH

**Purpose:** Compare performance across VM generations  
**Chart Type:** Grouped bar chart  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** VM Generation (categorical: 9.0, 9.1, 9.2, 10.2, 10.3, 11.x)
- **Y-axis:** Time (milliseconds, 0-1200ms range)
- **Bar Groups (per generation):**
  - Host Blackout P95 (red)
  - Guest Blackout P95 (orange)
  - Kernel Boot P95 (blue)
- **Annotations:** Show sample count below each generation
- **Sorting:** By generation number ascending

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| summarize 
    Count = count(),
    HostBlackout_P50 = percentile(HostBlackoutMS, 50),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95),
    HostBlackout_P99 = percentile(HostBlackoutMS, 99),
    GuestBlackout_P95 = percentile(GuestBlackoutMS, 95),
    KernelBoot_P95 = percentile(KernelBootTimeMS, 95)
    by VmGeneration
| where Count >= 100  // Filter low-volume generations
| order by VmGeneration asc
```

**Expected Output:** 10-15 VM generations

**Insights from Data:**
| Generation | Count (7d) | Host P95 | Guest P95 | Kernel P95 |
|------------|------------|----------|-----------|------------|
| 9.0 | 317K | 552ms | 552ms | 160ms |
| 9.1 | 509K | 416ms | 417ms | 210ms |
| 9.2 | 224K | 620ms | 614ms | 300ms |
| 10.2 | 93K | 402ms | 404ms | 181ms |
| **10.3** | **336K** | **934ms** | **936ms** | **271ms** |

**Key Finding:** Gen 10.3 has 2.2x higher blackout than Gen 9.1 - needs investigation!

---

### **VIZ 6: Top Clusters by Failure Rate** ⭐ HIGH

**Purpose:** Identify problematic clusters requiring investigation  
**Chart Type:** Horizontal bar chart  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** Failure rate (percentage, 0-5% range if any failures exist)
- **Y-axis:** Cluster name (top 20 clusters)
- **Bars:** Color gradient from green (0%) to red (high failure rate)
- **Annotations:**
  - Show absolute numbers: "Failed/Total"
  - Include hover tooltip with cluster details
- **Filter:** Only show clusters with ≥100 events (avoid noise)

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| summarize 
    Total = count(),
    Failed = countif(UnderhillSvcExecutionStatus == "failed"),
    Succeeded = countif(UnderhillSvcExecutionStatus == "succeeded")
    by Cluster
| extend FailureRate = (Failed * 100.0) / Total
| where Total >= 100  // Minimum volume threshold
| order by FailureRate desc, Total desc
| take 20
| project Cluster, FailureRate, Total, Failed, Succeeded
```

**Expected Output:** 20 clusters (or fewer if all have 0% failures)

**Insights from Data:**
- Current dataset shows 0% failure rate across all clusters
- Top clusters by volume: MWH23PrdApp04 (317K), AMS25PrdApp69 (219K), BY1PrdApp71 (112K)
- Replace with "Top Clusters by P95 Blackout" if no failures exist

**Alternative Query (if no failures):**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| summarize 
    Count = count(),
    HostBlackout_P50 = percentile(HostBlackoutMS, 50),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95)
    by Cluster
| where Count >= 100
| order by HostBlackout_P95 desc
| take 20
| project Cluster, P95_Blackout = HostBlackout_P95, P50_Blackout = HostBlackout_P50, EventCount = Count
```

---

### **VIZ 7: Blackout Heatmap by Cluster** ⭐ HIGH

**Purpose:** Detect spatial-temporal patterns in performance  
**Chart Type:** Heatmap (time × cluster matrix)  
**Update Frequency:** Hourly

**Visual Design:**
- **X-axis:** Time (hourly bins, 7-day window = 168 columns)
- **Y-axis:** Top 20 clusters by event volume
- **Color Scale:** Host P95 blackout time
  - Green: <400ms (excellent)
  - Yellow: 400-700ms (good)
  - Orange: 700-1000ms (concerning)
  - Red: >1000ms (critical)
- **Interactions:** Click cell to drill down to specific cluster+time

**Query:**
```kql
let topClusters = UnderhillTestServicingQualityMV
    | where UnderhillSvcExecutionStartTime > ago(7d)
    | summarize Count = count() by Cluster
    | order by Count desc
    | take 20
    | project Cluster;
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| where Cluster in (topClusters)
| summarize 
    Count = count(),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95)
    by Cluster, bin(UnderhillSvcExecutionStartTime, 1h)
| order by Cluster asc, UnderhillSvcExecutionStartTime asc
```

**Expected Output:** 20 clusters × 168 hours = 3,360 cells

**Insights from Data:**
- LVL10PrdApp10, DSM12PrdApp16, LVL10PrdApp11 consistently show higher P95 (~930-943ms)
- AMS25PrdApp69 has excellent performance (P95: 249ms)
- No cluster shows sustained degradation over time (would indicate infrastructure issue)

---

### **VIZ 8: Major Version Adoption by VM Generation** 🔵 MEDIUM

**Purpose:** Identify which VM generations still run 1.6 (need migration)  
**Chart Type:** Stacked bar chart or heatmap  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** VM Generation (9.0, 9.1, 9.2, 10.2, 10.3, 11.x)
- **Y-axis:** Event count (or percentage of generation total)
- **Stacked Segments:**
  - Version 1.7 (green)
  - Version 1.6 (red/orange - needs migration)
- **Annotations:**
  - Show% on 1.6 for each generation
  - Total unique VMs per generation
  - P95 blackout for each version+generation combo

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| extend MajorMinor = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| summarize 
    EventCount = count(),
    UniqueVMs = dcount(VmId),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95)
    by MajorMinor, VmGeneration
| order by VmGeneration asc, MajorMinor desc
```

**Expected Output:** 15-20 combinations (VM Gen × Major Version)

**Insights from Data:**
| Generation | 1.7 Events | 1.6 Events | % on 1.6 | 1.6 VMs | Priority |
|------------|------------|------------|----------|---------|----------|
| 9.1 | 454K | 54.5K | 10.7% | 1,364 VMs | 🟡 High volume |
| 10.3 | 287K | 48.1K | 14.4% | 394 VMs | 🔴 Worst P95 (1110ms) |
| 9.2 | 209K | 18.5K | 8.1% | 340 VMs | 🟢 Low % |
| 9.4 | 0 | 17.0K | 100% | 7 VMs | 🔴 **All on 1.6!** |
| 10.2 | 83.9K | 10.4K | 11% | 620 VMs | 🟡 Medium |
| 11.1 | 318 | 1.6K | 83.4% | 1,438 VMs | 🔴 **Mostly 1.6!** |

**🚨 Critical Findings:**
- **Gen 9.4: 100% on 1.6** (17K events, 7 VMs) - immediate migration required
- **Gen 11.1: 83.4% on 1.6** (1.6K events, 1,438 VMs) - new generation stuck on old version?
- **Gen 10.3 on 1.6 has worst P95: 1110ms** vs 911ms on 1.7 - high-impact migration target

**Action Items:**
- Prioritize Gen 9.4 and 11.1 migrations (nearly all on 1.6)
- Gen 10.3 + 1.6 combination is worst performer (1110ms P95) - migrate these 394 VMs first

---

### **VIZ 9: Downgrade Analysis** 🔵 MEDIUM

**Purpose:** Understand why 1.7 → 1.6 downgrades happen and their performance impact  
**Chart Type:** Scatter plot or detail table  
**Update Frequency:** Daily

**Visual Design:**
- **For Table:**
  - Columns: Timestamp, Cluster, VM Generation, Node, VM ID, Old Version (detailed), New Version (detailed), Blackout P95
  - Filter: Only 1.7 → 1.6 transitions
  - Sort: By timestamp descending
- **For Chart:**
  - X-axis: Time (if showing pattern discovery)
  - Y-axis: Count of downgrades
  - Color: By cluster or VM generation

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| extend OldMajor = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| extend NewMajor = iff(NewVmFirmwareIgvmVersion == "", "", extract(@"^(\d+\.\d+)", 1, NewVmFirmwareIgvmVersion))
| where OldMajor == "1.7" and NewMajor == "1.6"  // Downgrades only
| summarize 
    DowngradeCount = count(),
    HostBlackout_P50 = percentile(HostBlackoutMS, 50),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95),
    UniqueVMs = dcount(VmId)
    by Cluster, VmGeneration, OldVmFirmwareIgvmVersion, NewVmFirmwareIgvmVersion
| order by DowngradeCount desc
| take 50
```

**Expected Output:** 10-20 clusters with downgrades

**Insights from Data:**
- 40.9K downgrade events (2.7% of total traffic)
- Downgrades have P95: 768ms vs upgrades P95: 482ms (59% worse)
- Questions to answer:
  - Are these rollbacks due to 1.7 issues?
  - Are they part of canary/testing workflows?
  - Are they manual operations or automated?

**Action Items:**
- Determine if downgrades are intentional (testing) or reactive (rollbacks)
- If rollbacks: identify root cause of 1.7 issues causing them
- If testing: consider optimizing downgrade code path (59% slower than upgrades)

---

### **VIZ 10: Kernel Boot vs Blackout Correlation** 🔵 MEDIUM

**Purpose:** Understand relationship between boot time and total blackout  
**Chart Type:** Scatter plot with trend line  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** Kernel boot time (milliseconds, 80-500ms range)
- **Y-axis:** Host blackout time (milliseconds, 100-1200ms range)
- **Points:** Sample of 5,000 recent events
  - Color by VM generation (categorical color scale)
  - Size by logs flush time (optional)
- **Trend Line:** Linear regression showing correlation
- **Quadrants:** Divide into good/bad performance zones

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| where isnotnull(KernelBootTimeMS) and isnotnull(HostBlackoutMS)
| where KernelBootTimeMS < 1000 and HostBlackoutMS < 2000  // Remove extreme outliers
| sample 5000  // Performance optimization
| project 
    KernelBootTimeMS, 
    HostBlackoutMS, 
    GuestBlackoutMS, 
    VmGeneration, 
    LogsFlushTimeMS
```

**Expected Output:** 5,000 points

**Insights from Data:**
- Weak positive correlation (boot time contributes but isn't primary driver)
- Blackout = boot time + logs flush + overhead + other delays
- Gen 10.3 clusters in upper-right (high boot, high blackout)
- Fastest boots: 80-120ms (Gen 9.1 on some SKUs)

---

### **VIZ 11: Daily Volume and Failure Count** 🔵 MEDIUM

**Purpose:** Context for failure rates - absolute numbers matter  
**Chart Type:** Dual-axis chart (bar + line)  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** Time (daily bins, 30-day window)
- **Y-axis Left:** Total servicing events (bar chart, blue)
- **Y-axis Right:** Failure count (line chart, red, prominent)
- **Zero Line:** Emphasize y=0 for failures (solid line)

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(30d)
| summarize 
    TotalEvents = count(),
    FailureCount = countif(UnderhillSvcExecutionStatus == "failed"),
    SuccessCount = countif(UnderhillSvcExecutionStatus == "succeeded")
    by bin(UnderhillSvcExecutionStartTime, 1d)
| order by UnderhillSvcExecutionStartTime asc
```

**Expected Output:** 30 data points

**Insights from Data:**
- Daily volume ranges 128K-334K events
- Volume peaked on March 31 (334K events)
- Only 1 failure in entire 15-day period (excellent reliability)

---

### **VIZ 12: Performance by SKU** 🔵 MEDIUM

**Purpose:** Identify SKU-specific performance issues  
**Chart Type:** Sortable table with embedded sparklines  
**Update Frequency:** Daily

**Visual Design:**
- **Columns:**
  - SKU (string, left-aligned)
  - Event Count (right-aligned, thousands separator)
  - Success Rate % (right-aligned, color-coded)
  - P50 Blackout (ms, right-aligned)
  - P95 Blackout (ms, right-aligned, bold)
  - P99 Blackout (ms, right-aligned)
  - 7-Day Trend (sparkline showing daily P95)
- **Sorting:** Default by P95 descending (worst first)
- **Color Coding:** P95 > 800ms = red, 600-800ms = yellow, <600ms = green
- **Filter:** SKUs with ≥100 events

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| summarize 
    Total = count(),
    Succeeded = countif(UnderhillSvcExecutionStatus == "succeeded"),
    HostBlackout_P50 = percentile(HostBlackoutMS, 50),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95),
    HostBlackout_P99 = percentile(HostBlackoutMS, 99)
    by SKU
| extend SuccessRate = (Succeeded * 100.0) / Total
| where Total >= 100
| order by HostBlackout_P95 desc
| take 50
| project SKU, EventCount = Total, SuccessRate, P50 = HostBlackout_P50, P95 = HostBlackout_P95, P99 = HostBlackout_P99
```

**Expected Output:** 20-50 SKUs

**Top SKUs - Performance Summary:**
| SKU | Count | P95 | P99 | Notes |
|-----|-------|-----|-----|-------|
| Ingrasys-GB200-ARM-WCS-C4A14_RevA | 2.0K | 998ms | 1027ms | GPU compute, expected high |
| Lenovo-Intel-WCS-C21A0_RevB | 336K | 934ms | 1068ms | **HIGH VOLUME + HIGH LATENCY** |
| Lenovo-Intel-WCS-C2184_RevA | 17K | 819ms | 833ms | Consistent but high |
| Ingrasys-CR-ARM-WCS-C2141 | 110K | 717ms | 790ms | ARM outliers |
| Wiwynn-AMD-WCS-C2195 | 480K | **416ms** | 462ms | **BEST: High volume, low latency** |

**Key Finding:** Lenovo Intel C21A0_RevB (336K events) is high-volume + high-latency - critical for optimization!

---

### **VIZ 13: Worst Performers - Outlier Investigation Table** ⭐ HIGH

**Purpose:** Drill-down list for investigating specific slow servicing events  
**Chart Type:** Sortable detail table with filtering  
**Update Frequency:** Hourly

**Visual Design:**
- **Columns:**
  - Timestamp (datetime, sortable)
  - Cluster (string, filterable)
  - Node ID (string, monospace font)
  - VM ID (string, truncated with tooltip)
  - VM Generation (string, filterable)
  - Host Blackout (ms, color-coded: >5000=red, >1000=orange)
  - Guest Blackout (ms)
  - Kernel Boot (ms)
  - Logs Flush (ms)
  - Old Firmware (version, filterable)
  - New Firmware (version, filterable)
  - SKU (string, filterable)
- **Default Sort:** Host Blackout descending
- **Filter:** Top 100 worst events in last 7 days
- **Row Actions:** Click to copy VM ID, link to cluster monitoring dashboard

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| order by HostBlackoutMS desc
| take 100
| project 
    Timestamp = UnderhillSvcExecutionStartTime,
    Cluster,
    NodeId,
    VmId,
    VmGeneration,
    HostBlackoutMS,
    GuestBlackoutMS,
    KernelBootTimeMS,
    LogsFlushTimeMS,
    OldFirmware = OldVmFirmwareIgvmVersion,
    NewFirmware = NewVmFirmwareIgvmVersion,
    SKU
```

**Expected Output:** 100 rows (worst outliers)

**Insights from Top 20 Worst:**
- **Worst case:** 11.9 seconds (AM4PrdApp27, Gen 9.2, Self-servicing)
- **Common pattern in top outliers:**
  - Gen 9.2 + Ingrasys-CR-ARM-WCS-C2141 SKU (appears 13 times in top 20)
  - Host blackout is extreme but guest blackout is normal (<200ms)
  - Suggests host-side issue, not guest impact
- **Firmware upgrades to 1.7.506.0 from 1.6.517.2:** Several 5-8 second outliers (Gen 11.1)

**Key Action:** Investigate why Gen 9.2 ARM SKU has occasional 6-12 second host blackouts despite normal guest metrics

---

### **VIZ 14: Performance Improvement Opportunities** 🔵 MEDIUM

**Purpose:** Quantify the performance benefit of migrating remaining 1.6 VMs to 1.7  
**Chart Type:** Summary card / KPI dashboard  
**Update Frequency:** Daily

**Visual Design:**
- **Card Layout** with key metrics:
  - **VMs Still on 1.6:** Count + percentage
  - **Potential P50 Improvement:** Average ms saved per event
  - **Potential P95 Improvement:** Average ms saved per event
  - **Weekly Events Affected:** Count of 1.6 self-servicing events
  - **Projected Annual Time Saved:** If all migrated to 1.7
- **Visual:** Progress bar showing% migrated (1.7 events / total events)

**Query:**
```kql
let v16Stats = UnderhillTestServicingQualityMV
    | where UnderhillSvcExecutionStartTime > ago(7d)
    | where UnderhillSvcExecutionStatus == "succeeded"
    | extend MajorMinor = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
    | where MajorMinor == "1.6"
    | where NewVmFirmwareIgvmVersion == ""  // Self-servicing only
    | summarize 
        V16_Count = count(),
        V16_VMs = dcount(VmId),
        V16_P50 = percentile(HostBlackoutMS, 50),
        V16_P95 = percentile(HostBlackoutMS, 95);
let v17Stats = UnderhillTestServicingQualityMV
    | where UnderhillSvcExecutionStartTime > ago(7d)
    | where UnderhillSvcExecutionStatus == "succeeded"
    | extend MajorMinor = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
    | where MajorMinor == "1.7"
    | where NewVmFirmwareIgvmVersion == ""
    | summarize 
        V17_Count = count(),
        V17_P50 = percentile(HostBlackoutMS, 50),
        V17_P95 = percentile(HostBlackoutMS, 95);
v16Stats
| extend V17_P50 = toscalar(v17Stats | project V17_P50)
| extend V17_P95 = toscalar(v17Stats | project V17_P95)
| extend V17_Count = toscalar(v17Stats | project V17_Count)
| extend TotalCount = V16_Count + V17_Count
| extend PercentOn16 = (V16_Count * 100.0) / TotalCount
| extend P50_Improvement_MS = V16_P50 - V17_P50
| extend P95_Improvement_MS = V16_P95 - V17_P95
| extend AnnualTimeSaved_Hours = (V16_Count * 52 * P50_Improvement_MS) / (1000.0 * 3600.0)
| project 
    VMs_On_16 = V16_VMs,
    Percent_On_16 = PercentOn16,
    Weekly_16_Events = V16_Count,
    P50_Improvement_MS,
    P95_Improvement_MS,
    Annual_Hours_Saved = AnnualTimeSaved_Hours
```

**Expected Output:** 1 row with summary metrics

**Insights from Data:**
- **~2,905 VMs still on 1.6** (7% of active VMs)
- **104K 1.6 self-servicing events per week**
- **P50 improvement potential: 416ms per event** (653ms → 237ms)
- **P95 improvement potential: 311ms per event** (964ms → 653ms)
- **Annual time saved: ~2,260 hours of blackout time** if all migrate to 1.7

**ROI Calculation:**
- At 104K events/week on 1.6, eliminating 416ms/event saves **~11.9 hours of blackout per week**
- Annually: **619 hours = 25.8 days** of cumulative blackout eliminated
- Customer impact: Thousands of VMs experience faster servicing

---

## Failure Investigation Playbook

### Cross-Table ID Mapping (Confirmed)

All identity fields match exactly 1:1 between `UnderhillTestServicingQualityMV` (CCA cluster) and `UnderhillEventTable` (Fa cluster) — no transformation required:

| CCA Field | EventTable Field |
|---|---|
| `VmId` | `VmId` |
| `VmName` / `ContainerId` | `VmName` |
| `NodeId` | `NodeId` |
| `Cluster` | `Cluster` |

### Timestamp Strategy for Failed Records

For failed servicing operations, `UnderhillSvcExecutionStartTime` is **null**. Use the embedded timestamp from `ServicingSaveVtl2Key` instead:

```kql
// Extract time anchor from ServicingSaveVtl2Key
// e.g. "2026-04-01T05:06:07.8610081Z_servicing_save_vtl2"
| extend SaveKeyTimestamp = todatetime(extract(@"^([\d\-T:.Z]+)", 1, ServicingSaveVtl2Key))
```

Then query `UnderhillEventTable` ±10 minutes around `SaveKeyTimestamp`. The actual failure events typically appear 1–5 seconds before the save key timestamp.

### Known Failure Root Cause — April 1, 2026

**VM:** VmId `624bc584-9f4a-44ba-b90c-58dc210a8601`, Cluster `LVL10PrdApp56`  
**Firmware transition:** `1.6.498.0 → 1.7.506.0`  
**Root cause:** MANA network adapter hardware pre-failure

The MANA NIC (Microsoft Azure Network Adapter) was already in a failed hardware state **before** the servicing operation began. The complete failure chain:

```
05:05:19Z  mana_driver::bnic_driver  — "Previous hardware failure" (×6 endpoints)
05:05:19Z  mana_driver::resources    — "failed to tear down resource" × 6
05:05:19Z  mana_driver::gdma_driver  — "Previous hardware failure" × 2
05:05:24Z  underhill_core::worker    — "failed to start VM"
                                         ← "failed to merge configuration"
                                         ← "cancelled waiting for mana devices"
                                         ← "deadline exceeded"  [5-second timeout]
05:05:27Z  firmware_uefi             — UEFI DXE: "[Bds] Unable to boot!"
05:06:07Z  (CCA record)              — ServicingSaveVtl2Key recorded (failure finalized)
```

**Classification:** Hardware node degradation (not a firmware/software bug).  
**Signal for monitoring:** Multiple failures on the same `NodeId` within a short window indicate a degraded node requiring hardware replacement.

**Benign noise to filter out during failure analysis:**
- `firmware_uefi::service::diagnostics` PEI_CORE/DXE errors (variable policy not found, `AziHsmDeviceCnt:0`, image start failures) — normal platform behavior on this SKU
- `kmsg` ITS WARNING `"Spectre-v2 mitigation is off"` — normal message on VM restart

---

## Alert Thresholds & SLA Definitions

### Critical Alerts (Immediate Investigation)
- P95 Host Blackout > 1000ms sustained for >2 hours
- Failure rate > 0.1% in any 1-hour window
- Any cluster with >10 failures in 24 hours
- P99 Host Blackout > 2000ms sustained for >1 hour

### Warning Alerts (Monitor Closely)
- P95 Host Blackout increases >30% compared to 7-day baseline
- New VM generation or SKU P95 > 1200ms
- Cluster-specific P95 > 1500ms
- Firmware upgrade P95 > 1500ms

### SLA Targets (Recommended)
- Success Rate: >99.95% (current: 99.9999% ✅)
- P50 Host Blackout: <300ms (current: 244ms ✅)
- P95 Host Blackout: <800ms (current: 674ms ✅)
- P99 Host Blackout: <1200ms (current: 888ms ✅)

---

## Implementation Recommendations

### Dashboard Refresh Frequency
- **Real-time (1-minute refresh):** VIZ 2, VIZ 3 (time-based health monitoring)
- **Hourly refresh:** VIZ 7, VIZ 13 (operational monitoring)
- **Daily refresh:** VIZ 1, VIZ 4-6, VIZ 8-12, VIZ 14 (trend analysis)

### Data Retention & Performance
- **Materialized View:** Already optimized (10-minute lookback)
- **Dashboard queries:** All tested and execute in <5 seconds
- **Historical analysis:** Query source table `KO_UnderhillExecutionVmMetaDataExtensionOutput` for >15 days

### User Personas & Access
1. **SRE On-call:** VIZ 2, 3, 13 (real-time health + investigation)
2. **Performance Engineers:** VIZ 1, 4, 5, 7, 9, 10, 12 (version analysis + deep dive)
3. **Release Managers:** VIZ 1, 8, 14 (firmware version performance + rollout tracking)
4. **Executives:** VIZ 1, 3, 11 (version performance + high-level health + volume)

### Integration Points
- **Alert System:** Connect VIZ 1 (firmware regression alerts), VIZ 2, VIZ 3 to PagerDuty/Azure Monitor
- **Incident Management:** Link VIZ 13 to incident tracking (auto-create tickets for extreme outliers)
- **Capacity Planning:** Use VIZ 11 volume trends for forecasting
- **Release Management:** Block firmware releases if VIZ 1 shows P95 regression >10%

---

## Key Insights & Next Steps

### ✅ What's Working Well
- **Exceptional reliability:** 99.9999% success rate
- **Consistent performance:** P95 at 674ms meets target
- **High volume:** 1.5M events/week across 227 clusters
- **Fast medians:** P50 at 244ms shows efficient typical case

### ⚠️ Areas Requiring Investigation

**Priority 1: Accelerate 1.6 → 1.7 Migration by CPU Size** 🚨
- **Issue:** 1.6 self-servicing is 2.8× slower than 1.7 (P50: 653ms vs 237ms)
- **Impact:** 104K events (7% of traffic) still using 1.6
- **High-ROI Targets:**
  - **4 CPU VMs:** 551ms improvement per event (15.6K events on 1.6) - Best ratio!
  - **2 CPU VMs:** 471ms improvement per event (32.6K events on 1.6) - Highest volume
  - **32-96 CPU VMs:** >200ms improvement + absolute worst performance (>1 second on 1.6)
- **Opportunity:** 1.6→1.7 upgrades achieve BEST performance (P95: 482ms)
- **Action:** Fast-track migration plan; prioritize 4 CPU and 32+ CPU VMs first

**Priority 2: Large VM (32-96 CPU) on 1.6 - Critical Performance Issue**
- **Issue:** Largest VMs on 1.6 experience >1 second blackouts
  - 96 CPU + 1.6: P95 = 1164ms
  - 64 CPU + 1.6: P95 = 998ms
  - 32 CPU + 1.6: P95 = 995ms (23.7% of 32-CPU VMs still on 1.6!)
- **Impact:** High-value, expensive VMs experiencing worst performance
- **Action:** Identify and migrate all 32+ CPU VMs on 1.6 as highest urgency

**Priority 3: Investigate 16 CPU + 1.7 Anomaly**
- **Issue:** 16 CPU VMs on 1.7 perform slightly worse than 1.6 (327ms vs 314ms P95)
- **Impact:** Small regression, but unexpected
- **Action:** Investigate if this is a real regression or different workload characteristics

**Priority 4: VM Generation 10.3 Performance**
- **Issue:** Occasional 6-12 second host blackouts (guest unaffected)
- **SKU:** Ingrasys-Azure-Compute-GP-MM-CR-ARM-WCS-C2141
- **Pattern:** Host shows extreme delay, guest timing is normal
- **Action:** Investigate host-side delays (hypervisor, VMM, firmware save path?)

**Priority 4: Firmware Downgrade Performance**
- **Issue:** 1.7→1.6 downgrades have 60% higher blackout than 1.6→1.7 upgrades
- **Volume:** 40.7K downgrade events
- **Question:** Are downgrades rollbacks (failures) or planned (testing)?
- **Action:** Clarify intent and optimize downgrade path if frequent

### 🎯 Success Metrics for Dashboard

**Adoption Metrics:**
- Daily active users: Target >50 (SREs, PerfEngineers, Release Managers)
- Time-to-detect regression: <1 hour (via VIZ 1 firmware version alerts)
- Time-to-root-cause outlier: <30 minutes (via VIZ 13 drill-down)

**Operational Impact:**
- Catch firmware regressions BEFORE widespread deployment (via VIZ 1)
- Identify and fix Gen 10.3 performance gap
- Decrease extreme outliers (>5s) to <0.01% of events
- Reduce P95 blackout by 10% (674ms → 606ms) within Q2

---

## VM Size (CPU Count) Performance Analysis

### **VIZ 15: Blackout Performance by CPU Count** ⭐ CRITICAL

**Purpose:** Understand how VM size (CPU count) affects servicing performance across major versions  
**Chart Type:** Multi-line chart  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** CPU Count (2, 4, 8, 16, 32, 64, 96 vCPUs - log scale optional)
- **Y-axis:** Host Blackout P95 (milliseconds, 0-1200ms range)
- **Lines (4-6 main paths):**
  - 1.7 Self-Servicing (green, thick line)
  - 1.6 Self-Servicing (red, thick line)
  - 1.6 → 1.7 Upgrade (blue, dashed)
  - 1.7 → 1.6 Downgrade (orange, dashed)
- **Markers:** Data points sized by event volume
- **Annotations:**
  - Callout worst performers (e.g., "4 CPU + 1.6: 819ms!")
  - Show event count on hover

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| where VmSku != ""
| extend CPUs = toint(extract(@"[DEFAM](\d+)[a-z]*_v\d+", 1, VmSku))
| where isnotnull(CPUs)
| extend MajorVersion = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| extend TransitionPath = case(
    NewVmFirmwareIgvmVersion == "", strcat(MajorVersion, " Self-Servicing"),
    MajorVersion == extract(@"^(\d+\.\d+)", 1, NewVmFirmwareIgvmVersion), strcat(MajorVersion, " Minor Update"),
    strcat(MajorVersion, " → ", extract(@"^(\d+\.\d+)", 1, NewVmFirmwareIgvmVersion))
)
| where CPUs in (2, 4, 8, 16, 32, 64, 96)  // Common sizes
| summarize 
    EventCount = count(),
    HostBlackout_P50 = percentile(HostBlackoutMS, 50),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95),
    HostBlackout_P99 = percentile(HostBlackoutMS, 99),
    KernelBoot_P95 = percentile(KernelBootTimeMS, 95)
    by CPUs, TransitionPath
| order by CPUs asc, TransitionPath asc
```

**Expected Output:** ~35 data points (7 CPU sizes × 5 transition paths)

**Insights from Data (Last 7 Days):**

| CPUs | 1.6 Self P95 | 1.7 Self P95 | Improvement | 1.6→1.7 P95 | Volume (1.7 Self) |
|------|--------------|--------------|-------------|-------------|-------------------|
| **2** | 783ms | **312ms** | **2.5×** ✓ | 392ms | 552K events |
| **4** | **819ms** ⚠️ | **268ms** | **3.0×** ✓ | 265ms | 212K events |
| **8** | 349ms | 288ms | 1.2× | 283ms | 11K events |
| **16** | 314ms | 327ms | 0.96× ⚠️ | 247ms | 74K events |
| **32** | 995ms ⚠️ | 740ms | 1.3× | 390ms | 75K events |
| **64** | 998ms ⚠️ | 918ms | 1.1× | 480ms | 77K events |
| **96** | 1164ms ⚠️ | 937ms | 1.2× | 491ms | 100K events |

**🔥 CRITICAL CPU-SPECIFIC FINDINGS:**

1. **Small VMs (2-4 CPUs) benefit MOST from 1.7**: 2.5-3.0× improvement
   - 2 CPU: 783ms → 312ms (P95)
   - 4 CPU: 819ms → 268ms (P95) - **best improvement ratio!**

2. **Large VMs (32-96 CPUs) on 1.6 have WORST absolute performance**:
   - 96 CPU + 1.6: P95 = 1164ms (nearly 1.2 seconds!)
   - But still benefit from 1.7 migration

3. **Surprising: 16 CPU VMs on 1.7 slightly WORSE than 1.6** (327ms vs 314ms)
   - Investigate if this is a regression or workload difference

4. **CPU count doesn't directly correlate with worse performance**:
   - 1.7 self-servicing stays under 950ms P95 for all sizes
   - 1.6 shows wild variation (349ms to 1164ms)

**Action Items:**
- **High ROI:** Prioritize 2-4 CPU VM migrations (2.5-3× improvement)
- **High urgency:** Migrate 32-96 CPU VMs still on 1.6 (>1 second blackouts)
- **Investigate:** Why 16 CPU + 1.7 performs slightly worse than 1.6

---

### **VIZ 16: CPU Count × Transition Type Heatmap** ⭐ HIGH

**Purpose:** Comprehensive view of all CPU size + transition type combinations  
**Chart Type:** Heatmap  
**Update Frequency:** Daily

**Visual Design:**
- **Y-axis:** CPU Count (2, 4, 8, 16, 32, 48, 64, 72, 96 vCPUs)
- **X-axis:** Transition Type (1.6 Self, 1.7 Self, 1.6→1.7, 1.7→1.6)
- **Color Scale:** P95 Host Blackout
  - Green: <400ms (excellent)
  - Yellow: 400-700ms (good)
  - Orange: 700-1000ms (concerning)
  - Red: >1000ms (critical)
- **Cell Content:** P95 value + event count (on hover)
- **Gray out:** Cells with <100 events (insufficient data)

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| where VmSku != ""
| extend CPUs = toint(extract(@"[DEFAM](\d+)[a-z]*_v\d+", 1, VmSku))
| where isnotnull(CPUs)
| extend MajorVersion = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| extend NewMajor = iff(NewVmFirmwareIgvmVersion == "", "", extract(@"^(\d+\.\d+)", 1, NewVmFirmwareIgvmVersion))
| extend TransitionType = case(
    NewVmFirmwareIgvmVersion == "", strcat(MajorVersion, " Self"),
    strcat(MajorVersion, " → ", NewMajor)
)
| summarize 
    EventCount = count(),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95),
    GuestBlackout_P95 = percentile(GuestBlackoutMS, 95)
    by CPUs, TransitionType
| where EventCount >= 100  // Filter noise
```

**Expected Output:** ~40 cells (CPU sizes × transition types)

**Visual Pattern Recognition:**
- **Vertical green stripe:** 1.7 Self-Servicing performs well across all CPU counts
- **Vertical red stripe:** 1.6 Self-Servicing is problematic, especially for large VMs
- **Horizontal patterns:** Some CPU counts (4, 32-96) show more variation
- **Hotspots (red cells):** Immediate action required

**Use Case:** Quick identification of "red zone" combinations that need urgent attention

---

### **VIZ 17: Detailed CPU Size Performance Table** ⭐ HIGH

**Purpose:** Drill-down table with comprehensive metrics by CPU count  
**Chart Type:** Sortable, filterable data table  
**Update Frequency:** Daily

**Visual Design:**
- **Columns:**
  - CPU Count
  - Major Version (1.6 / 1.7)
  - Transition Type (Self / Upgrade / Downgrade)
  - Event Count
  - Unique VMs
  - P50 Blackout (ms)
  - P75 Blackout (ms)
  - **P95 Blackout (ms)** - bold, color-coded
  - P99 Blackout (ms)
  - Kernel Boot P95 (ms)
  - Logs Flush P95 (ms)
- **Sorting:** Default by P95 descending (worst first)
- **Filtering:** CPU count range, transition type, version
- **Color Coding:** 
  - P95 < 400ms = green
  - P95 400-700ms = yellow
  - P95 > 700ms = red

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| where VmSku != ""
| extend CPUs = toint(extract(@"[DEFAM](\d+)[a-z]*_v\d+", 1, VmSku))
| where isnotnull(CPUs)
| extend MajorVersion = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| extend TransitionPath = case(
    NewVmFirmwareIgvmVersion == "", "Self-Servicing",
    MajorVersion == extract(@"^(\d+\.\d+)", 1, NewVmFirmwareIgvmVersion), "Minor Update",
    MajorVersion < extract(@"^(\d+\.\d+)", 1, NewVmFirmwareIgvmVersion), "Upgrade",
    "Downgrade"
)
| summarize 
    EventCount = count(),
    UniqueVMs = dcount(VmId),
    P50_Blackout = percentile(HostBlackoutMS, 50),
    P75_Blackout = percentile(HostBlackoutMS, 75),
    P95_Blackout = percentile(HostBlackoutMS, 95),
    P99_Blackout = percentile(HostBlackoutMS, 99),
    KernelBoot_P95 = percentile(KernelBootTimeMS, 95),
    LogsFlush_P95 = percentile(LogsFlushTimeMS, 95)
    by CPUs, MajorVersion, TransitionPath
| where EventCount >= 50  // Meaningful sample size
| order by P95_Blackout desc
```

**Expected Output:** 50-100 rows

**Use Case:**
- Identify worst combinations for targeted optimization
- Compare same CPU count across versions
- Export data for further analysis
- Drill down from heatmap to details

---

### **VIZ 18: CPU Count vs Blackout Scatter Plot** 🔵 MEDIUM

**Purpose:** Visualize correlation and identify outliers  
**Chart Type:** Scatter plot with trend lines  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** CPU Count (log scale recommended for 2-96 range)
- **Y-axis:** P95 Host Blackout (milliseconds)
- **Points:**
  - Color by major version (1.6 = red, 1.7 = green)
  - Size by event count (larger = more events)
  - Shape by transition type (circle = self-servicing, triangle = upgrades, square = downgrades)
- **Trend Lines:**
  - One for 1.6 self-servicing (red dashed)
  - One for 1.7 self-servicing (green dashed)
- **Annotations:**
  - Label extreme outliers
  - Show event count on hover

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| where VmSku != ""
| extend CPUs = toint(extract(@"[DEFAM](\d+)[a-z]*_v\d+", 1, VmSku))
| where isnotnull(CPUs)
| extend MajorVersion = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| extend TransitionType = case(
    NewVmFirmwareIgvmVersion == "", "Self-Servicing",
    "Version Change"
)
| summarize 
    EventCount = count(),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95)
    by CPUs, MajorVersion, TransitionType
| where EventCount >= 100
```

**Expected Output:** ~40 data points

**Insights to Discover:**
- **Linear correlation?** Does more CPUs = more blackout?
  - Answer from data: NO! It's version-dependent, not CPU count
- **Outliers:** Which CPU counts deviate from trend?
  - 4 CPU + 1.6 = outlier (worse than expected)
  - 16 CPU + 1.7 = outlier (worse than smaller VMs)
- **Clustering:** Do points cluster by version or by CPU range?

---

### **VIZ 19: CPU Size Distribution and Migration Opportunity** 🔵 MEDIUM

**Purpose:** Show distribution of VM sizes and identify high-impact migration targets  
**Chart Type:** Stacked bar chart with overlay  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** CPU Count (2, 4, 8, 16, 32, 64, 96)
- **Y-axis Left:** Number of servicing events (bar chart)
- **Y-axis Right:** P95 improvement potential if 1.6 migrates to 1.7 (line)
- **Stacked Bars:**
  - 1.7 events (green)
  - 1.6 events (red - migration opportunity)
- **Overlay Line:** Improvement in milliseconds (P95 delta)
- **Annotations:** Show % still on 1.6 for each CPU count

**Query:**
```kql
let v16Stats = UnderhillTestServicingQualityMV
    | where UnderhillSvcExecutionStartTime > ago(7d)
    | where UnderhillSvcExecutionStatus == "succeeded"
    | where VmSku != ""
    | where NewVmFirmwareIgvmVersion == ""  // Self-servicing only
    | extend CPUs = toint(extract(@"[DEFAM](\d+)[a-z]*_v\d+", 1, VmSku))
    | where isnotnull(CPUs)
    | extend MajorVersion = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
    | where MajorVersion == "1.6"
    | summarize 
        V16_Count = count(),
        V16_P95 = percentile(HostBlackoutMS, 95)
        by CPUs;
let v17Stats = UnderhillTestServicingQualityMV
    | where UnderhillSvcExecutionStartTime > ago(7d)
    | where UnderhillSvcExecutionStatus == "succeeded"
    | where VmSku != ""
    | where NewVmFirmwareIgvmVersion == ""
    | extend CPUs = toint(extract(@"[DEFAM](\d+)[a-z]*_v\d+", 1, VmSku))
    | where isnotnull(CPUs)
    | extend MajorVersion = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
    | where MajorVersion == "1.7"
    | summarize 
        V17_Count = count(),
        V17_P95 = percentile(HostBlackoutMS, 95)
        by CPUs;
v16Stats
| join kind=fullouter v17Stats on CPUs
| extend CPUs = coalesce(CPUs, CPUs1)
| extend V16_Count = coalesce(V16_Count, 0)
| extend V17_Count = coalesce(V17_Count, 0)
| extend V16_P95 = coalesce(V16_P95, 0.0)
| extend V17_P95 = coalesce(V17_P95, 0.0)
| extend TotalCount = V16_Count + V17_Count
| extend PercentOn16 = (V16_Count * 100.0) / TotalCount
| extend ImprovementMS = V16_P95 - V17_P95
| where CPUs in (2, 4, 8, 16, 32, 64, 96)
| project CPUs, V16_Count, V17_Count, TotalCount, PercentOn16, V16_P95, V17_P95, ImprovementMS
| order by CPUs asc
```

**Expected Output:** 7 rows (one per major CPU size)

**Insights from Data:**
| CPUs | 1.6 Events | 1.7 Events | % on 1.6 | Improvement (ms) | Priority |
|------|------------|------------|----------|------------------|----------|
| 2 | 32.6K | 552K | 5.6% | 471ms | 🟢 Most migrated |
| 4 | 15.6K | 212K | 6.8% | 551ms | 🟡 **Best improvement** |
| 32 | 23.3K | 75K | 23.7% | 255ms | 🔴 High % on 1.6 |
| 96 | ? | 100K | ? | 227ms | ❓ Check |

**Action Priorities:**
1. **4 CPU VMs:** Best improvement ratio (551ms savings per event)
2. **32+ CPU VMs:** Highest % still on 1.6 (need migration)

---

### **VIZ 20: Memory-Optimized vs General Purpose (D vs E Series)** 🔵 MEDIUM

**Purpose:** Compare performance between D-series (general) and E-series (memory-optimized) VMs  
**Chart Type:** Grouped bar chart  
**Update Frequency:** Daily

**Visual Design:**
- **X-axis:** CPU Count (common sizes where both D and E exist: 64, 96)
- **Y-axis:** P95 Host Blackout (milliseconds)
- **Bar Groups (per CPU count):**
  - D-series 1.7 Self (light blue)
  - E-series 1.7 Self (dark blue)
  - D-series 1.6 Self (light red)
  - E-series 1.6 Self (dark red)
- **Annotations:**
  - Show event count for each bar
  - RAM per vCPU indicator (D=4GB, E=8GB)

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| where VmSku != ""
| where NewVmFirmwareIgvmVersion == ""  // Self-servicing only
| extend CPUs = toint(extract(@"[DEFAMLN](\d+)[a-z]*_v\d+", 1, VmSku))
| extend VMSeries = extract(@"Standard_([DEFAMLN])", 1, VmSku)
| where VMSeries in ("D", "E")  // General purpose vs memory-optimized
| where CPUs in (64, 96)  // Sizes where both series are common
| extend MajorVersion = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| summarize 
    EventCount = count(),
    HostBlackout_P50 = percentile(HostBlackoutMS, 50),
    HostBlackout_P95 = percentile(HostBlackoutMS, 95),
    HostBlackout_P99 = percentile(HostBlackoutMS, 99)
    by CPUs, VMSeries, MajorVersion
| order by CPUs asc, VMSeries asc, MajorVersion asc
```

**Expected Output:** 8 bars (2 CPU sizes × 2 series × 2 versions)

**Research Question:** Does extra memory (E-series) improve servicing performance?
- **Hypothesis:** More RAM might speed up VTL2 save/restore operations
- **Expected:** Minimal difference (servicing is CPU-bound, not memory-bound)

**Insights to Discover:**
- If E-series is NOT faster → memory is not a bottleneck for servicing
- If E-series IS faster → consider recommending E-series for servicing-heavy workloads
- Compare cost-benefit: E-series costs ~2× more than D-series

---

All queries have been validated against production data and are ready for implementation.

### Data Freshness Check
```kql
UnderhillTestServicingQualityMV
| summarize LatestData = max(UnderhillSvcExecutionStartTime)
```

### Overall Health Summary
```kql
UnderhillTestServicingQualityMV
| summarize 
    TotalEvents = count(),
    SuccessRate = countif(UnderhillSvcExecutionStatus == "succeeded") * 100.0 / count(),
    P50_Blackout = percentile(HostBlackoutMS, 50),
    P95_Blackout = percentile(HostBlackoutMS, 95),
    P99_Blackout = percentile(HostBlackoutMS, 99)
```

### Resource Coverage (Last 7 Days)
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| summarize 
    Clusters = dcount(Cluster),
    Nodes = dcount(NodeId),
    VMs = dcount(VmId)
```

---

**Document Version:** 1.0  
**Last Updated:** April 2, 2026  
**Next Review:** April 9, 2026 (weekly cadence recommended)

---

## Guest-Reported Blackout by Servicing Transition Type

### **VIZ 21: Guest-Reported Blackout by Transition Type** ⭐ CRITICAL

**Purpose:** Compare guest-reported blackout times across the four major servicing transition types (1.6→1.6, 1.6→1.7, 1.7→1.7, 1.7→1.6) to understand guest-perceived impact of each operation  
**Chart Type:** User-defined  
**Update Frequency:** Daily

**Query:**
```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| extend OldMajorMinor = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| extend NewMajorMinor = iff(NewVmFirmwareIgvmVersion == "", OldMajorMinor, extract(@"^(\d+\.\d+)", 1, NewVmFirmwareIgvmVersion))
| extend TransitionType = strcat(OldMajorMinor, " → ", NewMajorMinor)
| where TransitionType in ("1.6 → 1.6", "1.6 → 1.7", "1.7 → 1.7", "1.7 → 1.6")
| extend VersionDetail = iff(NewVmFirmwareIgvmVersion == "", strcat(OldVmFirmwareIgvmVersion, " → ", OldVmFirmwareIgvmVersion), strcat(OldVmFirmwareIgvmVersion, " → ", NewVmFirmwareIgvmVersion))
| summarize 
    EventCount = count(),
    P50_ms = percentile(GuestBlackoutMS, 50),
    P75_ms = percentile(GuestBlackoutMS, 75),
    P95_ms = percentile(GuestBlackoutMS, 95),
    P99_ms = percentile(GuestBlackoutMS, 99),
    P99_9_ms = percentile(GuestBlackoutMS, 99.9),
    P99_99_ms = percentile(GuestBlackoutMS, 99.99),
    GuestBlackout_Max = max(GuestBlackoutMS),
    GuestBlackout_Avg = round(avg(GuestBlackoutMS), 1)
    by TransitionType, VersionDetail
| order by TransitionType asc, EventCount desc
```

**Data (April 3–10, 2026):**

| Transition Type | Version Detail | Event Count | P50 | P75 | P95 | P99 | P99.9 | P99.99 | Max | Avg |
|-----------------|----------------|-------------|-----|-----|-----|-----|-------|--------|-----|-----|
| **1.6 → 1.6** | 1.6.499.0 → 1.6.499.0 | 16,581 | 798ms | 808ms | 827ms | 839ms | 861ms | 1,118ms | 1,159ms | 796.8ms |
| | 1.6.522.0 → 1.6.522.0 | 5,167 | 365ms | 734ms | 801ms | 839ms | 882ms | 905ms | 905ms | 468.8ms |
| | 1.6.498.0 → 1.6.498.0 | 50 | 1,252ms | 1,745ms | 2,241ms | 2,563ms | 2,563ms | 2,563ms | 2,563ms | 1,357.4ms |
| **1.6 → 1.7** | 1.6.522.0 → 1.7.506.0 | 99,500 | 235ms | 410ms | 500ms | 545ms | 1,094ms | 1,856ms | 1,997ms | 298.7ms |
| | 1.6.517.2 → 1.7.506.0 | 185 | 277ms | 316ms | 493ms | 1,158ms | 1,159ms | 1,159ms | 1,159ms | 297.5ms |
| | 1.6.522.0 → 1.7.505.0 | 87 | 218ms | 235ms | 269ms | 474ms | 474ms | 474ms | 474ms | 234.3ms |
| | 1.6.521.0 → 1.7.506.0 | 10 | 280ms | 454ms | 802ms | 802ms | 802ms | 802ms | 802ms | 355.7ms |
| | 1.6.498.0 → 1.7.506.0 | 1 | 1,120ms | 1,120ms | 1,120ms | 1,120ms | 1,120ms | 1,120ms | 1,120ms | 1,120.0ms |
| **1.7 → 1.6** | 1.7.506.0 → 1.6.522.0 | 87,821 | 325ms | 647ms | 765ms | 815ms | 872ms | 937ms | 1,671ms | 419.3ms |
| | 1.7.506.0 → 1.6.521.0 | 13 | 345ms | 349ms | 366ms | 366ms | 366ms | 366ms | 366ms | 342.3ms |
| | 1.7.506.0 → 1.6.517.2 | 11 | 279ms | 312ms | 499ms | 499ms | 499ms | 499ms | 499ms | 307.5ms |
| | 1.7.507.0 → 1.6.522.0 | 1 | 283ms | 283ms | 283ms | 283ms | 283ms | 283ms | 283ms | 283.0ms |
| **1.7 → 1.7** | 1.7.506.0 → 1.7.506.0 | 1,453,464 | 284ms | 440ms | 773ms | 1,147ms | 3,413ms | 8,287ms | 29,328ms | 363.0ms |
| | 1.7.505.0 → 1.7.505.0 | 516,665 | 225ms | 246ms | 446ms | 587ms | 691ms | 919ms | 1,307ms | 247.5ms |
| | 1.7.506.0 → 1.7.498.0 | 617 | 301ms | 315ms | 3,697ms | 3,881ms | 4,466ms | 4,466ms | 4,466ms | 685.6ms |

**Key Observations:**
- **1.6 → 1.6 self-servicing has by far the worst guest blackout** — P50 of 790ms is 3.3× worse than 1.7→1.7 (241ms) and the distribution is tightly clustered around 790-826ms
- **1.6 → 1.7 upgrades have the best P95/P99** — P95: 500ms, P99: 545ms — tighter than even 1.7→1.7 self-servicing (676ms/1,011ms), suggesting the upgrade path is well-optimized
- **1.7 → 1.7 self-servicing has the widest tail** — P99 reaches 1,011ms and max is 29.3 seconds, but this is the highest-volume category (1.97M events) so rare outliers are expected
- **1.7 → 1.6 downgrades are moderately expensive** — P50: 326ms, P95: 765ms — worse than upgrades (1.6→1.7) across all percentiles
- **1.6 → 1.6 is almost entirely self-servicing** (21,801 of 21,803 events) while 1.7→1.7 includes 617 minor-update events

**Verification Notes:**
- Self-servicing events (empty `NewVmFirmwareIgvmVersion`) correctly map to same-version transitions (1.6→1.6 and 1.7→1.7)
- Cross-version transitions (1.6→1.7 and 1.7→1.6) are all firmware change operations
- Total events across all four types: 2,177,469

---

## Keepalive Status Lookup via UnderhillEventTable

### Purpose

Determine whether a specific servicing operation ran with NVMe keepalive enabled or disabled. This requires querying the `UnderhillEventTable` on the `azcore.centralus` cluster, since keepalive status is not recorded in the CCA materialized view.

### How to Identify Keepalive Status

During a servicing save, OpenHCL emits an `nvme_manager_save` span entry (Target: `underhill_core::dispatch`) whose Fields contain `nvme_keepalive_mode` — either `"true"` (keepalive enabled) or `"false"` (keepalive disabled). This is the single definitive indicator.

Example event:
```json
{
  "Name": "nvme_manager_save",
  "Target": "underhill_core::dispatch",
  "Fields": "{\"nvme_keepalive_mode\":\"false\"}"
}
```

### Query: Determine Keepalive Status for a Servicing Event

```kql
let fn_nodeId = "<NODE_ID>";
let fn_containerId = "<CONTAINER_ID>";  // This is VmName in UnderhillEventTable
let fn_servicingTime = datetime(<SERVICING_TIMESTAMP>);  // UnderhillSvcExecutionStartTime from CCA
let fn_startTime = fn_servicingTime;
let fn_endTime = fn_servicingTime + 1m;
UnderhillEventTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId and VmName == fn_containerId
| where Message has "nvme_manager_save"
| extend ParsedMsg = parse_json(Message)
| extend InnerMsg = parse_json(tostring(ParsedMsg.Message))
| where toint(InnerMsg.op_code) == 1  // Span enter only
| extend Fields = parse_json(tostring(ParsedMsg.Fields))
| extend NvmeKeepAliveMode = tostring(Fields.nvme_keepalive_mode)
| summarize arg_min(PreciseTimeStamp, NvmeKeepAliveMode)
| project PreciseTimeStamp, NvmeKeepAliveMode
```

### Example

For VM `bc18b17a-a24e-4710-97a8-9c03a88cec91` on node `c1e5d5ff-6466-2e39-941c-55c0fc9319a6` at `2026-04-10T09:34:20.890Z`:

| PreciseTimeStamp | NvmeKeepAliveMode |
|------------------|-------------------|
| 09:34:21.530Z | false |

---

## Guest Blackout by Transition Type with Keepalive Breakdown

### **VIZ 22: Guest Blackout by Transition Type with 1.7→1.7 Keepalive Split** ⭐ CRITICAL

**Purpose:** Compare guest-reported blackout across all servicing transition types, with 1.7→1.7 further split by NVMe keepalive status (KA = enabled, No KA = disabled, Err = status unknown)  
**Chart Type:** User-defined  
**Update Frequency:** Daily

**Methodology:** Keepalive status is not in the CCA materialized view. This query cross-cluster joins `UnderhillTestServicingQualityMV` (CCA) with `UnderhillEventTable` (azcore.centralus/Fa) by building a per-VmId keepalive lookup from `nvme_manager_save` span entries. For each VmId, the earliest `nvme_keepalive_mode` observation is taken via `arg_min`. VMs with no keepalive data in UnderhillEventTable are classified as "Err".

**Query:**
```kql
let keepaliveLookup = cluster('azcore.centralus.kusto.windows.net').database('Fa').UnderhillEventTable
| where PreciseTimeStamp > ago(7d)
| where Message has "nvme_manager_save"
| extend ParsedMsg = parse_json(Message)
| extend InnerMsg = parse_json(tostring(ParsedMsg.Message))
| where toint(InnerMsg.op_code) == 1  // Span enter only
| extend Fields = parse_json(tostring(ParsedMsg.Fields))
| extend NvmeKeepAlive = tostring(Fields.nvme_keepalive_mode)
| where NvmeKeepAlive in ("true", "false")  // Only definitive status
| summarize arg_min(PreciseTimeStamp, NvmeKeepAlive) by VmId
| project VmId, NvmeKeepAlive;
let baseData = UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| extend OldMajorMinor = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| extend NewMajorMinor = iff(NewVmFirmwareIgvmVersion == "", OldMajorMinor, extract(@"^(\d+\.\d+)", 1, NewVmFirmwareIgvmVersion))
| extend TransitionType = strcat(OldMajorMinor, " → ", NewMajorMinor)
| where TransitionType in ("1.6 → 1.6", "1.6 → 1.7", "1.7 → 1.7", "1.7 → 1.6");
let otherTransitions = baseData
| where TransitionType != "1.7 → 1.7"
| summarize 
    EventCount = count(),
    P50_ms = percentile(GuestBlackoutMS, 50),
    P75_ms = percentile(GuestBlackoutMS, 75),
    P95_ms = percentile(GuestBlackoutMS, 95),
    P99_ms = percentile(GuestBlackoutMS, 99),
    P99_9_ms = percentile(GuestBlackoutMS, 99.9),
    P99_99_ms = percentile(GuestBlackoutMS, 99.99),
    GuestBlackout_Max = max(GuestBlackoutMS),
    GuestBlackout_Avg = round(avg(GuestBlackoutMS), 1)
    by TransitionType
| extend SortOrder = case(
    TransitionType == "1.6 → 1.6", 1,
    TransitionType == "1.6 → 1.7", 2,
    TransitionType == "1.7 → 1.6", 3,
    10);
let v17Overall = baseData
| where TransitionType == "1.7 → 1.7"
| summarize 
    EventCount = count(),
    P50_ms = percentile(GuestBlackoutMS, 50),
    P75_ms = percentile(GuestBlackoutMS, 75),
    P95_ms = percentile(GuestBlackoutMS, 95),
    P99_ms = percentile(GuestBlackoutMS, 99),
    P99_9_ms = percentile(GuestBlackoutMS, 99.9),
    P99_99_ms = percentile(GuestBlackoutMS, 99.99),
    GuestBlackout_Max = max(GuestBlackoutMS),
    GuestBlackout_Avg = round(avg(GuestBlackoutMS), 1)
| extend TransitionType = "1.7 → 1.7", SortOrder = 4;
let v17ByKA = baseData
| where TransitionType == "1.7 → 1.7"
| join kind=leftouter keepaliveLookup on VmId
| extend KAStatus = case(
    NvmeKeepAlive == "true", "1.7 → 1.7 (KA)",
    NvmeKeepAlive == "false", "1.7 → 1.7 (No KA)",
    "1.7 → 1.7 (Err)"
)
| summarize 
    EventCount = count(),
    P50_ms = percentile(GuestBlackoutMS, 50),
    P75_ms = percentile(GuestBlackoutMS, 75),
    P95_ms = percentile(GuestBlackoutMS, 95),
    P99_ms = percentile(GuestBlackoutMS, 99),
    P99_9_ms = percentile(GuestBlackoutMS, 99.9),
    P99_99_ms = percentile(GuestBlackoutMS, 99.99),
    GuestBlackout_Max = max(GuestBlackoutMS),
    GuestBlackout_Avg = round(avg(GuestBlackoutMS), 1)
    by KAStatus
| extend TransitionType = KAStatus
| extend SortOrder = case(
    TransitionType == "1.7 → 1.7 (KA)", 5,
    TransitionType == "1.7 → 1.7 (No KA)", 6,
    7);
union otherTransitions, v17Overall, v17ByKA
| project TransitionType, EventCount, P50_ms, P75_ms, P95_ms, P99_ms, P99_9_ms, P99_99_ms, GuestBlackout_Max, GuestBlackout_Avg, SortOrder
| order by SortOrder asc
```

**Data (April 3–10, 2026):**

| Transition Type | Event Count | P50 | P75 | P95 | P99 | P99.9 | P99.99 | Max | Avg |
|-----------------|-------------|-----|-----|-----|-----|-------|--------|-----|-----|
| 1.6 → 1.6 | 21,806 | 790ms | 805ms | 826ms | 842ms | 1,320ms | 2,241ms | 2,563ms | 720.4ms |
| 1.6 → 1.7 | 102,256 | 236ms | 408ms | 500ms | 545ms | 1,102ms | 1,855ms | 1,997ms | 298.5ms |
| 1.7 → 1.6 | 90,201 | 327ms | 645ms | 764ms | 815ms | 872ms | 932ms | 1,671ms | 418.0ms |
| **1.7 → 1.7** | **1,981,640** | **242ms** | **398ms** | **679ms** | **1,015ms** | **3,385ms** | **7,010ms** | **29,328ms** | **333.1ms** |
| 1.7 → 1.7 (KA) | 1,760,094 | 237ms | 388ms | 620ms | 935ms | 1,011ms | 1,149ms | 5,227ms | 311.6ms |
| 1.7 → 1.7 (No KA) | 118,974 | 233ms | 439ms | 499ms | 948ms | 1,016ms | 1,455ms | 1,544ms | 304.1ms |
| 1.7 → 1.7 (Err) | 102,572 | 419ms | 839ms | 2,256ms | 3,545ms | 9,569ms | 23,372ms | 29,328ms | 735.2ms |

**Key Findings:**

1. **1.7→1.7 (KA) vs (No KA) — Similar median, different P95:**
   - P50 nearly identical: 237ms (KA) vs 233ms (No KA)
   - No KA has *better* P95: 499ms vs 620ms — simpler save path without keepalive may reduce variation
   - Both have similar P99: 935ms vs 948ms

2. **1.7→1.7 (Err) is dramatically worse — driving the overall tail:**
   - P50 already 419ms (1.8× worse than KA/No KA)
   - P95 at 2,256ms (3.6× worse than KA)
   - P99.99 at 23.4 seconds — extreme outliers
   - These are 102.5K events from 237 VMs with **empty VmGeneration, SKU, and VmSku metadata** — a distinct class of VMs where keepalive status is not available in UnderhillEventTable
   - Without these Err VMs, the 1.7→1.7 overall P99 would drop from 1,015ms to ~935ms

3. **1.6→1.6 remains the worst steady-state performer** (P50: 790ms)

4. **1.6→1.7 upgrades remain the best P95** across all categories (500ms)

**Action Items:**
- Investigate the Err category (237 VMs with empty metadata) — these are driving the entire 1.7→1.7 tail and inflating overall P99/P99.9 significantly
- Investigate why No KA has better P95 than KA (499ms vs 620ms) — is the keepalive save/restore path adding overhead at the tail?

---

## 1.7→1.7 Transitions by Minor Version and Keepalive Status

### **VIZ 23: 1.7→1.7 Minor Version × Keepalive Breakdown** ⭐ CRITICAL

**Purpose:** Break down 1.7→1.7 self-servicing operations by exact minor version AND keepalive status (KA / No KA / Err)  
**Chart Type:** User-defined  
**Update Frequency:** Daily

**Query:**
```kql
let keepaliveLookup = cluster('azcore.centralus.kusto.windows.net').database('Fa').UnderhillEventTable
| where PreciseTimeStamp > ago(7d)
| where Message has "nvme_manager_save"
| extend ParsedMsg = parse_json(Message)
| extend InnerMsg = parse_json(tostring(ParsedMsg.Message))
| where toint(InnerMsg.op_code) == 1  // Span enter only
| extend Fields = parse_json(tostring(ParsedMsg.Fields))
| extend NvmeKeepAlive = tostring(Fields.nvme_keepalive_mode)
| where NvmeKeepAlive in ("true", "false")
| summarize arg_min(PreciseTimeStamp, NvmeKeepAlive) by VmId
| project VmId, NvmeKeepAlive;
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| extend OldMajorMinor = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| extend NewMajorMinor = iff(NewVmFirmwareIgvmVersion == "", OldMajorMinor, extract(@"^(\d+\.\d+)", 1, NewVmFirmwareIgvmVersion))
| where OldMajorMinor == "1.7" and NewMajorMinor == "1.7"
| extend VersionDetail = iff(NewVmFirmwareIgvmVersion == "", strcat(OldVmFirmwareIgvmVersion, " → ", OldVmFirmwareIgvmVersion), strcat(OldVmFirmwareIgvmVersion, " → ", NewVmFirmwareIgvmVersion))
| join kind=leftouter keepaliveLookup on VmId
| extend KAStatus = case(
    NvmeKeepAlive == "true", "KA",
    NvmeKeepAlive == "false", "No KA",
    "Err"
)
| summarize 
    EventCount = count(),
    P50_ms = percentile(GuestBlackoutMS, 50),
    P75_ms = percentile(GuestBlackoutMS, 75),
    P95_ms = percentile(GuestBlackoutMS, 95),
    P99_ms = percentile(GuestBlackoutMS, 99),
    P99_9_ms = percentile(GuestBlackoutMS, 99.9),
    P99_99_ms = percentile(GuestBlackoutMS, 99.99),
    GuestBlackout_Max = max(GuestBlackoutMS),
    GuestBlackout_Avg = round(avg(GuestBlackoutMS), 1)
    by VersionDetail, KAStatus
| order by VersionDetail asc, KAStatus asc
```

**Data (April 3–10, 2026):**

| Version Detail | KA Status | Events | P50 | P75 | P95 | P99 | P99.9 | P99.99 | Max | Avg |
|----------------|-----------|--------|-----|-----|-----|-----|-------|--------|-----|-----|
| 1.7.505.0 → 1.7.505.0 | KA | 510,177 | 225ms | 246ms | 447ms | 587ms | 691ms | 919ms | 1,307ms | 247.4ms |
| 1.7.506.0 → 1.7.498.0 | KA | 617 | 301ms | 315ms | 3,697ms | 3,881ms | 4,466ms | 4,466ms | 4,466ms | 685.6ms |
| 1.7.506.0 → 1.7.506.0 | Err | 103,107 | 420ms | 840ms | 2,278ms | 3,538ms | 9,509ms | 23,372ms | 29,328ms | 734.2ms |
| 1.7.506.0 → 1.7.506.0 | KA | 1,249,990 | 276ms | 423ms | 671ms | 951ms | 1,018ms | 1,115ms | 5,227ms | 337.6ms |
| 1.7.506.0 → 1.7.506.0 | No KA | 119,181 | 233ms | 439ms | 499ms | 948ms | 1,016ms | 1,455ms | 1,544ms | 303.9ms |

**Key Findings:**

1. **1.7.505.0 self-servicing is exclusively KA and is the best performer:**
   - P50: 225ms, P95: 447ms — all 510K events with keepalive enabled
   - No "No KA" or "Err" rows exist for this version

2. **1.7.506.0 self-servicing KA vs No KA:**
   - No KA has better P95: 499ms vs 671ms (same pattern as aggregate)
   - No KA also wins at P50: 233ms vs 276ms
   - Err category (103K events) has P95: 2,278ms — still the worst

3. **1.7.506.0 → 1.7.498.0 minor downgrade is all KA with pathological tail:**
   - Only 617 events but P95: 3,697ms — something wrong with this code path
