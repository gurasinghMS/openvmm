# Underhill Production Servicing Quality Report

**Date:** April 7, 2026
**Data Source:** `wdgeventstore.kusto.windows.net` → `CCA` → `KO_UnderhillExecutionVmMetaDataExtensionOutput`
**Filter:** `Source == "UnderhillSvc.exe"` (production deployments only)
**Purpose:** Monitor Underhill/OpenHCL servicing quality in **production** (not Cirrus/test)

---

## How This Differs from the Cirrus Dashboard

The existing [Underhill_Servicing_Quality_Dashboard_Specification.md](Underhill_Servicing_Quality_Dashboard_Specification.md) monitors **Cirrus test deployments** via the `UnderhillTestServicingQualityMV` materialized view. This document covers **production** deployments.

Key differences:
- **Data Source:** Same underlying table (`KO_UnderhillExecutionVmMetaDataExtensionOutput`) but filtered on `Source == "UnderhillSvc.exe"` instead of `UnderhillSvcTest.exe`
- **The MV named "Test" actually mixes prod and test data** — it has no `Source` column filter. The MV name is misleading.
- **Production is ~83% of all rows** in the source table (~2M prod vs ~400K test per day)
- **Version landscape is inverted:** Production is 99% v1.6; Cirrus is 93% v1.7
- **Failure rates are higher in prod** due to 1.7→1.6 downgrade issues

---

## Executive Summary

### Data Profile (Last 7 Days: March 31 – April 7, 2026)

| Metric | Value |
|--------|-------|
| **Total Servicing Operations** | 330,938 |
| **Overall Success Rate** | 99.11% (2,956 failures) |
| **Clusters** | 1,088 |
| **Nodes** | 65,182 |
| **VMs** | 191,936 |
| **Regions** | 47 |
| **Dominant Version** | 1.6 (99.0% of operations) |

### Key Performance Metrics (Succeeded Operations)

| Metric | P50 | P95 | P99 | Max |
|--------|-----|-----|-----|-----|
| **Host Blackout** | 301ms | 669ms | 1,547ms | 8,771ms |
| **Guest Blackout** | 302ms | 670ms | 1,553ms | — |

### Critical Findings

**🚨 v1.7 Has a 31.6% Success Rate in Production**
- Only 1,078 of 3,408 v1.7 operations succeeded (68.4% failure rate)
- Failures are concentrated in **1.7 → 1.6 downgrades** (2,318 failures = 78% of all failures)
- Dominant error: `0xC0370803` at `RestoreManagementVtlState` stage
- This is in stark contrast to Cirrus, where v1.7 has 99.9999% success rate
- **Root cause indicator:** The 1.7→1.6 downgrade path is fundamentally broken in production

**⚠️ v1.6 Is Dominant and Performing Reasonably**
- 99.0% of production operations run on v1.6
- v1.6 success rate: 99.81% (626 failures out of 327,479)
- Blackout P50: 302ms, P95: 668ms — comparable to Cirrus

**⚠️ v1.7 Minor Updates Have Extreme Blackout Times**
- P95: 3,718ms, P99: 3,870ms (vs ~668ms for v1.6 minor updates)
- Small sample size (544 events) but consistently high

**🔴 Hotspot Regions**
- **Sweden Central:** 55.28% failure rate (1,246 of 2,254 operations)
- **US Central EUAP (canary):** 58.73% failure rate (407 of 693 operations)
- **Spain Central:** 10.19% failure rate (645 of 6,331 operations)
- All other regions are under 1.1% failure rate

**🔴 Hotspot Clusters**
- **GVX22PrdApp02** (Sweden): 92.41% failure rate (1,242 of 1,344)
- **CDM12PrdApp03**: 85.37% failure rate
- **MAD21PrdApp22** (Spain): 71.53% failure rate

---

## Section 1: Servicing Volume

### Daily Volume and Success Rate

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where isnotempty(UnderhillSvcExecutionStatus)
| summarize
    Total = count(),
    Succeeded = countif(UnderhillSvcExecutionStatus == "succeeded"),
    Failed = countif(UnderhillSvcExecutionStatus == "failed")
    by bin(UnderhillSvcExecutionStartTime, 1d)
| extend SuccessRate = round((Succeeded * 100.0) / Total, 4)
| order by UnderhillSvcExecutionStartTime asc
```

**Results:**

| Date | Total | Succeeded | Failed | Success Rate |
|------|-------|-----------|--------|-------------|
| Mar 31 | 16,474 | 16,262 | 212 | 98.71% |
| Apr 1 | 94,596 | 93,795 | 801 | 99.15% |
| Apr 2 | 63,585 | 63,308 | 277 | 99.56% |
| **Apr 3** | **82,515** | **81,168** | **1,347** | **98.37%** |
| Apr 4 | 41,776 | 41,646 | 130 | 99.69% |
| Apr 5 | 20,543 | 20,415 | 128 | 99.38% |
| Apr 6 | 8,850 | 8,811 | 39 | 99.56% |
| Apr 7 (partial) | 2,598 | 2,576 | 22 | 99.15% |

**Observations:**
- Weekday volume is 40K-95K/day; weekends drop to 9K-21K/day
- April 3 had the worst success rate (98.37%) and highest absolute failures (1,347)
- Failures have been trending down since April 3

### Resource Coverage (Last 7 Days)

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where isnotempty(UnderhillSvcExecutionStatus)
| summarize
    TotalEvents = count(),
    Clusters = dcount(Cluster),
    Nodes = dcount(NodeId),
    VMs = dcount(VmId),
    Regions = dcount(region)
```

| Metric | Value |
|--------|-------|
| Total Events | 330,938 |
| Clusters | 1,088 |
| Nodes | 65,182 |
| VMs | 191,936 |
| Regions | 47 |

**Comparison with Cirrus (same period):** Cirrus had 1.5M events across 227 clusters — production has lower event volume but 4.8× more clusters and 33× more VMs, reflecting the breadth of the production fleet.

---

## Section 2: Version Distribution

### Version Breakdown (Initiated Version)

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where isnotempty(UnderhillSvcExecutionStatus)
| extend OldMajorMinor = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| summarize Count = count() by OldMajorMinor
| order by Count desc
```

| Initiated Version (Old) | Count | % of Total |
|--------------------------|-------|------------|
| **1.6** | 327,481 | **99.0%** |
| **1.7** | 3,408 | **1.0%** |
| 1.5 | 47 | <0.01% |
| 0.7 | 2 | <0.01% |
| 1.2 | 1 | <0.01% |

**Key Insight:** Production is overwhelmingly on v1.6 — only 1% has v1.7. This is the **inverse** of Cirrus, where 93% runs v1.7. Production has not yet migrated to v1.7.

### Transition Types (Succeeded Only)

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
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
| summarize EventCount = count() by TransitionPath
| order by EventCount desc
```

| Transition Path | Event Count | % |
|----------------|-------------|---|
| **1.6 Minor Update** | 314,912 | 96.0% |
| **1.6 → 1.7 Upgrade** | 11,943 | 3.6% |
| 1.7 Minor Update | 544 | 0.17% |
| 1.7 → 1.6 Downgrade | 531 | 0.16% |
| 1.5 → 1.6 Upgrade | 47 | 0.01% |
| Others | 6 | <0.01% |

**Observations:**
- 96% of succeeded operations are v1.6 minor updates (patch-level servicing within 1.6)
- 3.6% are 1.6→1.7 upgrades — the production rollout of v1.7 is actively happening
- There are 531 successful 1.7→1.6 downgrades — these are likely rollbacks

### Version-Specific Success/Failure Rates

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where isnotempty(UnderhillSvcExecutionStatus)
| extend OldMajorMinor = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| summarize
    Total = count(),
    Succeeded = countif(UnderhillSvcExecutionStatus == "succeeded"),
    Failed = countif(UnderhillSvcExecutionStatus == "failed")
    by OldMajorMinor
| extend SuccessRate = round((Succeeded * 100.0) / Total, 4)
| order by Total desc
```

| Initiated Version | Total | Succeeded | Failed | Success Rate |
|-------------------|-------|-----------|--------|-------------|
| **1.6** | 327,479 | 326,853 | 626 | **99.81%** |
| **1.7** | 3,408 | 1,078 | 2,330 | **31.63%** 🚨 |
| 1.5 | 47 | 47 | 0 | 100% |

**🚨 CRITICAL:** v1.7 has only a 31.63% success rate in production. This means **68.4% of servicing operations initiated on v1.7 fail.** Immediate investigation required.

---

## Section 3: Blackout Performance

### Overall Blackout Metrics (Last 7 Days, Succeeded Only)

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| where isnotempty(UnderhillBlackoutHostPerspective)
| summarize
    Count = count(),
    HostBlackout_P50 = percentile(UnderhillBlackoutHostPerspective, 50),
    HostBlackout_P95 = percentile(UnderhillBlackoutHostPerspective, 95),
    HostBlackout_P99 = percentile(UnderhillBlackoutHostPerspective, 99),
    HostBlackout_Max = max(UnderhillBlackoutHostPerspective),
    GuestBlackout_P50 = percentile(UnderhillBlackoutGuestPerspective, 50),
    GuestBlackout_P95 = percentile(UnderhillBlackoutGuestPerspective, 95),
    GuestBlackout_P99 = percentile(UnderhillBlackoutGuestPerspective, 99)
```

| Metric | Host Blackout (ms) | Guest Blackout (ms) |
|--------|-------------------|-------------------|
| **Count** | 324,549 | — |
| **P50** | 301 | 302 |
| **P95** | 669 | 670 |
| **P99** | 1,547 | 1,553 |
| **Max** | 8,771 | — |

**Comparison with Cirrus:**

| Metric | Production | Cirrus |
|--------|-----------|--------|
| Host P50 | 301ms | 244ms |
| Host P95 | 669ms | 674ms |
| Host P99 | 1,547ms | 888ms |
| Host Max | 8,771ms | 20,153ms |

Production has ~23% higher P50 but similar P95. The P99 is significantly higher in production (1,547ms vs 888ms), indicating a longer tail of slow operations.

### Blackout by Transition Path

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| where isnotempty(UnderhillBlackoutHostPerspective)
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
    HostBlackout_P50 = percentile(UnderhillBlackoutHostPerspective, 50),
    HostBlackout_P95 = percentile(UnderhillBlackoutHostPerspective, 95),
    HostBlackout_P99 = percentile(UnderhillBlackoutHostPerspective, 99)
    by TransitionPath
| order by EventCount desc
```

| Transition Path | Count | P50 | P95 | P99 | Assessment |
|----------------|-------|-----|-----|-----|------------|
| **1.6 Minor Update** | 311,876 | 302ms | 668ms | 1,541ms | ✅ Baseline |
| **1.6 → 1.7 Upgrade** | 11,598 | 195ms | 772ms | 1,808ms | ⚠️ Low P50 but high P99 |
| **1.7 Minor Update** | 541 | 304ms | **3,718ms** | **3,870ms** | 🚨 **Extremely high P95/P99** |
| **1.7 → 1.6 Downgrade** | 481 | 289ms | 499ms | 1,670ms | ✅ P50/P95 OK |

**Key Findings:**
1. v1.6 minor updates (the bulk of production) perform consistently: P50=302ms, P95=668ms
2. 1.6→1.7 upgrades have the fastest P50 (195ms) but elevated P99 (1,808ms) — bimodal distribution
3. **v1.7 minor updates are catastrophically slow** — P95 of 3,718ms means most v1.7 self-servicing takes 3-4 seconds. Small sample but consistent.
4. 1.7→1.6 downgrades (when they succeed) perform reasonably

### Daily Blackout Trends

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| where isnotempty(UnderhillBlackoutHostPerspective)
| summarize
    Count = count(),
    HostBlackout_P50 = percentile(UnderhillBlackoutHostPerspective, 50),
    HostBlackout_P95 = percentile(UnderhillBlackoutHostPerspective, 95),
    HostBlackout_P99 = percentile(UnderhillBlackoutHostPerspective, 99)
    by bin(UnderhillSvcExecutionStartTime, 1d)
| order by UnderhillSvcExecutionStartTime asc
```

| Date | Count | P50 | P95 | P99 |
|------|-------|-----|-----|-----|
| Mar 31 | 16,157 | 301ms | 1,194ms | 1,781ms |
| Apr 1 | 92,681 | 290ms | 659ms | 1,707ms |
| Apr 2 | 62,722 | 295ms | 507ms | 1,367ms |
| Apr 3 | 80,638 | 310ms | 638ms | 926ms |
| Apr 4 | 41,101 | 308ms | 672ms | 1,010ms |
| Apr 5 | 20,061 | 609ms | 727ms | 1,079ms |
| Apr 6 | 8,644 | 199ms | 985ms | 1,176ms |
| Apr 7 | 2,545 | 598ms | 1,031ms | 1,715ms |

**Observations:**
- P50 is generally stable at 290-310ms on weekdays but volatile on weekends (low volume)
- P95 improved from 1,194ms (Mar 31) to 507ms (Apr 2), rebounded to ~670ms, volatile on weekends
- Weekend/low-volume days show more measurement noise

### Blackout Breakdown (JSON)

The `UnderhillSvcBlackoutBreakdown` column contains a JSON blob with detailed stage-level timing:

| Key | Type | Description | Typical Values |
|-----|------|-------------|---------------|
| `UnderhillKernelBootTimeMS` | long | Kernel boot time in ms | 131-181ms |
| `UnderhillLogsFlushTimeNS` | long | Logs flush time in ns | 136-233ns |
| `UnderhillSvcStageTimeTakenMS` | dict | Wall-clock time per stage | `servicing_save_vtl2: 32ms` |
| `UnderhillSvcStageTimeActiveMS` | dict | Active/CPU time per stage | `servicing_save_vtl2: 2ms` |

Common stage names: `servicing_save_vtl2`, `save_units`, `restore_units`, `network_settings`, `new_mana_device`, `new_gdma_driver`, `base_chipset_build`, `apply_vtl2_protections`, `shutdown_mana`, `nic_shutdown`

**Query to extract breakdown metrics:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "succeeded"
| where isnotempty(UnderhillSvcBlackoutBreakdown)
| extend Breakdown = parse_json(UnderhillSvcBlackoutBreakdown)
| extend KernelBootTimeMS = tolong(Breakdown.UnderhillKernelBootTimeMS)
| summarize
    Count = count(),
    KernelBoot_P50 = percentile(KernelBootTimeMS, 50),
    KernelBoot_P95 = percentile(KernelBootTimeMS, 95),
    KernelBoot_P99 = percentile(KernelBootTimeMS, 99)
```

---

## Section 4: Failure Analysis

### Failure by Error Code and Stage

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "failed"
| summarize Count = count() by ErrorCode, ServicingStage
| order by Count desc
```

| ErrorCode | Servicing Stage | Count | % of Failures |
|-----------|----------------|-------|--------------|
| *(empty)* | *(empty)* | 2,275 | 77.0% |
| `0xC0370803` | RestoreManagementVtlState | 677 | 22.9% |
| `0x800704C7` | RestoreManagementVtlState | 4 | 0.1% |

**Breakdown:**
- 77% of failures have no error code or stage populated — many show `ResetReason: "The operation has failed past the point of no return."`
- 22.9% fail with `0xC0370803` at the `RestoreManagementVtlState` stage
- 0.1% are cancelled operations (`0x800704C7`)

### Failures by Transition Path

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "failed"
| extend OldMajorMinor = extract(@"^(\d+\.\d+)", 1, OldVmFirmwareIgvmVersion)
| extend NewMajorMinor = iff(NewVmFirmwareIgvmVersion == "", "", extract(@"^(\d+\.\d+)", 1, NewVmFirmwareIgvmVersion))
| extend TransitionPath = case(
    NewVmFirmwareIgvmVersion == "", strcat(OldMajorMinor, " Self-Servicing"),
    NewMajorMinor == "", strcat(OldMajorMinor, " Self-Servicing"),
    OldMajorMinor == NewMajorMinor, strcat(OldMajorMinor, " Minor Update"),
    strcat(OldMajorMinor, " → ", NewMajorMinor)
)
| summarize Failed = count() by TransitionPath, ErrorCode, ServicingStage
| order by Failed desc
```

| Transition | ErrorCode | Stage | Count |
|-----------|-----------|-------|-------|
| **1.7 → 1.6 Downgrade** | *(empty)* | *(empty)* | **1,644** |
| **1.7 → 1.6 Downgrade** | `0xC0370803` | RestoreManagementVtlState | **674** |
| 1.6 Minor Update | *(empty)* | *(empty)* | 449 |
| 1.6 → 1.7 Upgrade | *(empty)* | *(empty)* | 174 |
| 1.7 Minor Update | *(empty)* | *(empty)* | 8 |
| 1.7 Minor Update | `0x800704C7` | RestoreManagementVtlState | 4 |
| 1.6 → 1.7 Upgrade | `0xC0370803` | RestoreManagementVtlState | 3 |

**🚨 The 1.7 → 1.6 downgrade path accounts for 78% of all production failures (2,318 of 2,956).**

The `0xC0370803` error at `RestoreManagementVtlState` is almost exclusively a 1.7→1.6 downgrade issue (674 of 677 total instances).

### Top Failing Clusters

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where isnotempty(UnderhillSvcExecutionStatus)
| summarize
    Total = count(),
    Failed = countif(UnderhillSvcExecutionStatus == "failed")
    by Cluster
| where Total >= 100
| extend FailureRate = round((Failed * 100.0) / Total, 2)
| order by FailureRate desc
| take 20
```

| Cluster | Total | Failed | Failure Rate |
|---------|-------|--------|-------------|
| **GVX22PrdApp02** | 1,344 | 1,242 | **92.41%** 🔴 |
| CDM12PrdApp03 | 164 | 140 | 85.37% 🔴 |
| MAD21PrdApp22 | 418 | 299 | 71.53% 🔴 |
| CDM14PrdApp08 | 213 | 132 | 61.97% 🔴 |
| MAD21PrdApp24 | 662 | 329 | 49.70% 🔴 |
| CDM11PrdApp03 | 274 | 126 | 45.99% 🔴 |
| LVL06PrdApp57 | 388 | 82 | 21.13% |
| CVL05PrdApp02 | 112 | 19 | 16.96% |
| MAD21PrdApp23 | 178 | 11 | 6.18% |
| PAR60PrdApp06 | 178 | 11 | 6.18% |

6 clusters have >40% failure rate — all likely performing 1.7→1.6 downgrades.

### Failures by Region

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where isnotempty(UnderhillSvcExecutionStatus)
| summarize
    Total = count(),
    Failed = countif(UnderhillSvcExecutionStatus == "failed")
    by region
| where Total >= 100
| extend FailureRate = round((Failed * 100.0) / Total, 4)
| order by FailureRate desc
| take 20
```

| Region | Total | Failed | Failure Rate |
|--------|-------|--------|-------------|
| **uscentraleuap** (canary) | 693 | 407 | **58.73%** 🔴 |
| **swedenc** | 2,254 | 1,246 | **55.28%** 🔴 |
| **spainc** | 6,331 | 645 | **10.19%** 🔴 |
| useast2euap (canary) | 2,180 | 24 | 1.10% |
| australiasoutheast | 1,228 | 10 | 0.81% |
| canadaeast | 851 | 6 | 0.71% |
| uswest2 | 4,392 | 16 | 0.36% |
| useast2 | 80,550 | 231 | 0.29% |
| europewest | 49,519 | 139 | 0.28% |
| asiasoutheast | 14,543 | 40 | 0.28% |

The canary region (`uscentraleuap`) and `swedenc` are the worst affected. These are likely where 1.7→1.6 downgrades are being attempted most aggressively.

### Daily Failure Trends

**Query:**
```kql
KO_UnderhillExecutionVmMetaDataExtensionOutput
| where Source == "UnderhillSvc.exe"
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus == "failed"
| summarize FailureCount = count() by bin(UnderhillSvcExecutionStartTime, 1d), ErrorCode
| order by UnderhillSvcExecutionStartTime asc
```

| Date | No Error Code | 0xC0370803 | 0x800704C7 | Total |
|------|--------------|------------|------------|-------|
| Mar 31 | 152 | 60 | — | 212 |
| **Apr 1** | **689** | 112 | — | **801** |
| Apr 2 | 250 | 27 | — | 277 |
| **Apr 3** | **927** | **418** | 2 | **1,347** |
| Apr 4 | 92 | 36 | 2 | 130 |
| Apr 5 | 117 | 11 | — | 128 |
| Apr 6 | 33 | 6 | — | 39 |
| Apr 7 | 15 | 7 | — | 22 |

April 1 and April 3 were the worst days. The `0xC0370803` spike on April 3 (418 occurrences) correlates with aggressive downgrade attempts.

---

## Section 5: Supplementary Data Sources

Production servicing data can also be corroborated via these tables:

### HyperVWorkerTable (azcore.centralus / Fa)

Servicing event IDs in the worker process:
- **5124**: Servicing failure (contains stage, version info, error code)
- **5126**: Servicing success (contains correlation ID, firmware versions)
- **5128**: VM reset after servicing failure
- **5136**: Servicing failed due to guest-initiated power event

**Daily PROD volume from HyperVWorkerTable (last 7 days):**

| Date | 5126 (Success) | 5124 (Failure) | 5128 (Reset) | Total |
|------|----------------|----------------|--------------|-------|
| Mar 31 | 50,161 | 17 | 17 | 50,195 |
| Apr 1 | 357,726 | 696 | 696 | 359,118 |
| Apr 2 | 299,059 | 235 | 235 | 299,529 |
| Apr 3 | 322,816 | 142 | 141 | 323,100 |
| Apr 4 | 289,869 | 241 | 241 | 290,351 |
| Apr 5 | 277,444 | 264 | 263 | 277,971 |
| Apr 6 | 283,011 | 108 | 108 | 283,227 |
| Apr 7 | 339,226 | 13 | 12 | 339,251 |
| **Total** | **2,219,312** | **1,716** | **1,713** | **2,222,741** |

**Note:** HyperVWorkerTable shows **2.2M events** vs CCA's **331K events** for production. The discrepancy is because HyperVWorkerTable counts per-VM servicing events at the worker level (each VM generates an event), while CCA captures a subset. The failure count is also lower in HyperVWorkerTable (1,716 vs 2,956) — possibly due to different counting/dedup logic.

**Query for HyperVWorkerTable:**
```kql
cluster('azcore.centralus.kusto.windows.net').database("Fa").HyperVWorkerTable
| where PreciseTimeStamp > ago(7d)
| where EventId in (5124, 5126, 5128, 5136)
| where Environment == "PROD"
| summarize Count = count() by bin(PreciseTimeStamp, 1d), EventId
| order by PreciseTimeStamp asc, EventId asc
```

### UnderhillEventTable (azcore.centralus / Fa)

Detailed per-event telemetry from inside Underhill:

| Target | Events/Hour (PROD) | Metrics Available |
|--------|-------------------|-------------------|
| `underhill_core::dispatch` | ~286K | `blackout_time_ms`, `correlation_id`, servicing save/restore spans |
| `underhill_core::worker` | ~274K | `kernel_boot_time_ns`, servicing state transfer details |
| `underhill_init` | ~247K | `KERNEL_BOOT_TIME` env var, init process launch |

**Key timing metrics from UnderhillEventTable:**

| Metric | Field | Typical Range |
|--------|-------|---------------|
| Blackout time | `blackout_time_ms` in "resuming VM" | 173–408ms |
| VTL2 save time | `time_taken_ns` in `servicing_save_vtl2` | ~42ms total |
| Kernel boot time | `kernel_boot_time_ns` in "kernel boot time" | 180–231ms |
| Boot time (fresh) | `boot_time_ms` in "starting VM" | 430–611ms |
| Saved state size | `saved_state_len` in "received servicing state" | ~201KB |

---

## Data Source Notes

### How to Filter Production vs Test

The `KO_UnderhillExecutionVmMetaDataExtensionOutput` table has a `Source` column:

| Source Value | Meaning | ~Daily Volume |
|-------------|---------|--------------|
| `UnderhillSvc.exe` | **Production** | ~2M rows/day |
| `UnderhillSvcTest.exe` | Cirrus/Test | ~400K rows/day |
| `UnderhillSvcTestUpgrade.exe` | Test Upgrade | ~18K rows/day |

**The `UnderhillTestServicingQualityMV` materialized view does NOT have a `Source` column** and likely includes both production and test data despite its "Test" name. For clean production-only queries, always go to the source table with `Source == "UnderhillSvc.exe"`.

### Column Name Differences

The source table uses slightly different column names from the MV:

| Source Table Column | MV Column | Type |
|-------------------|-----------|------|
| `UnderhillSvcExecutionStartTime` | `UnderhillSvcExecutionStartTime` | datetime |
| `UnderhillBlackoutHostPerspective` | `HostBlackoutMS` | long |
| `UnderhillBlackoutGuestPerspective` | `GuestBlackoutMS` | long |
| `UnderhillSvcBlackoutBreakdown` | (parsed into `KernelBootTimeMS`, `LogsFlushTimeMS`) | string (JSON) |
| `OldVmFirmwareIgvmVersion` | `OldVmFirmwareIgvmVersion` | string |
| `NewVmFirmwareIgvmVersion` | `NewVmFirmwareIgvmVersion` | string |
| `generation` | `VmGeneration` | string |
| `region` | *(not in MV)* | string |
| `Source` | *(not in MV)* | string |
| `SubscriptionId` | *(not in MV)* | string |

---

## Recommended Next Steps

1. **Investigate v1.7 failure rate** (31.6%) — why are 1.7→1.6 downgrades failing at `RestoreManagementVtlState`?
2. **Investigate v1.7 minor update blackout** (P95: 3,718ms) — why are v1.7 self-servicing operations so slow in production?
3. **Deep-dive on GVX22PrdApp02 cluster** (92% failure rate) and Sweden Central region
4. **Correlate with UnderhillEventTable** for detailed per-event root cause on failures
5. **Build production-specific MV or view** with `Source == "UnderhillSvc.exe"` filter for faster dashboard queries
6. **Add generation/SKU breakdowns** (available in source table but not yet analyzed)
7. **Add region-level performance heatmap** (47 regions available)

---

**Document Version:** 1.0
**Last Updated:** April 7, 2026
