# UnderhillTestServicingQualityMV

**Type:** Materialized View  
**Cluster:** `https://wdgeventstore.kusto.windows.net`  
**Database:** `CCA`  
**Full Path:** `wdgeventstore.kusto.windows.net` → `CCA` → `UnderhillTestServicingQualityMV`

## Description

Materialized view providing insights into Cirrus runs that execute servicing operations on Underhill/OpenVMM/OpenHCL. Tracks execution status, performance metrics (blackout times, boot times), firmware versions, and VM configuration during servicing operations.

**Materialization Details:**
- Source table: `KO_UnderhillExecutionVmMetaDataExtensionOutput`
- Lookback window: 600 seconds (10 minutes) based on `UnderhillSvcExecutionStartTime`
- Filters applied: Only includes firmware versions starting with "1.7" or upgrades from "1.6" to "1.7"
- Deduplicates records and parses timing breakdowns from JSON fields

**Performance Note:**
- For fast queries, query this materialized view directly instead of querying the underlying source table
- The materialized view is updated periodically and may not contain the absolute latest data
- If you need the most up-to-date information, query the source table `KO_UnderhillExecutionVmMetaDataExtensionOutput` directly (slower but current)

## Use Cases

- Track latest servicing test executions
- Monitor performance regressions in servicing operations
- Analyze boot time and blackout duration trends
- Identify failed servicing operations by cluster/node/VM
- Compare firmware version transitions

## Schema

| Column Name | Type | Description |
|-------------|------|-------------|
| `Cluster` | string | Azure cluster where the servicing operation ran |
| `NodeId` | string | Physical node identifier |
| `VmId` | string | Virtual machine identifier |
| `VmName` | string | Virtual machine name |
| `VmUniqueId` | string | Unique VM identifier across clusters |
| `ContainerId` | string | Container hosting the VM |
| `OldVmFirmwareIgvmVersion` | string | Firmware IGVM version before servicing |
| `NewVmFirmwareIgvmVersion` | string | Firmware IGVM version after servicing (empty string for self-servicing events) |
| `UnderhillSvcExecutionStatus` | string | Status of servicing execution (possible values: `"succeeded"`, `"failed"`) |
| `SKU` | string | VM SKU/size |
| `VmGeneration` | string | VM generation (Gen1, Gen2) |
| `KernelBootTimeMS` | long | Kernel boot time in milliseconds |
| `LogsFlushTimeMS` | long | Time to flush logs in milliseconds |
| `HostBlackoutMS` | long | Host-side blackout duration in milliseconds |
| `GuestBlackoutMS` | long | Guest-side blackout duration in milliseconds |
| `VmSku` | string | Virtual machine SKU details |
| `UnderhillSvcExecutionStartTime` | datetime | **Timestamp when servicing execution started** (use for latest data queries) |
| `ServicingSaveVtl2Key` | string | VTL2 save key for servicing operation |

## Important Columns for Common Queries

- **Timestamp queries:** `UnderhillSvcExecutionStartTime`
- **Status filtering:** `UnderhillSvcExecutionStatus` (values: `"succeeded"`, `"failed"` - note lowercase)
- **Performance analysis:** `KernelBootTimeMS`, `HostBlackoutMS`, `GuestBlackoutMS`, `LogsFlushTimeMS`
- **Version tracking:** `OldVmFirmwareIgvmVersion`, `NewVmFirmwareIgvmVersion`
- **Resource identification:** `Cluster`, `NodeId`, `VmId`, `VmName`

## Sample Queries

### Get latest data timestamp

```kql
UnderhillTestServicingQualityMV
| summarize LatestTimestamp = max(UnderhillSvcExecutionStartTime)
```

### Get recent failed executions

```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| where UnderhillSvcExecutionStatus != "succeeded"
| project UnderhillSvcExecutionStartTime, Cluster, NodeId, VmId, UnderhillSvcExecutionStatus, OldVmFirmwareIgvmVersion, NewVmFirmwareIgvmVersion
| take 20
```

### Analyze boot time distribution

```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(24h)
| where UnderhillSvcExecutionStatus == "succeeded"
| summarize 
    P50 = percentile(KernelBootTimeMS, 50),
    P95 = percentile(KernelBootTimeMS, 95),
    P99 = percentile(KernelBootTimeMS, 99),
    Count = count()
  by VmGeneration
```

### Track firmware version transitions

```kql
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStartTime > ago(7d)
| summarize Count = count() by OldVmFirmwareIgvmVersion, NewVmFirmwareIgvmVersion
| order by Count desc
| take 10
```

---

## Cross-Table Correlation with UnderhillEventTable

### Confirmed ID Mapping (Empirically Verified)

All identity fields match **exactly 1:1** across both tables — no transformation needed:

| CCA Column | EventTable Column | Notes |
|---|---|---|
| `VmId` | `VmId` | Exact GUID match (primary join key) |
| `VmName` / `ContainerId` | `VmName` | Same GUID; `VmName` is stable across servicing operations |
| `NodeId` | `NodeId` | Exact GUID match |
| `Cluster` | `Cluster` | Exact string match (e.g., `"LVL10PrdApp56"`) |

> **Note:** `VmName` (a GUID, not human-readable) does **not** change between servicing runs for the same VM. It is a stable identifier you can use to find all logs for a VM across time.

### Timestamp Strategy for Failure Cases

For **failed** servicing operations, `UnderhillSvcExecutionStartTime` is `null`. Use the embedded timestamp in `ServicingSaveVtl2Key` as the time anchor:

```kql
// Extract the timestamp from ServicingSaveVtl2Key
// Example value: "2026-04-01T05:06:07.8610081Z_servicing_save_vtl2"
| extend SaveKeyTimestamp = todatetime(extract(@"^([\d\-T:.Z]+)", 1, ServicingSaveVtl2Key))
```

Query EventTable **±10 minutes** around `SaveKeyTimestamp`. The actual failure events typically appear a few seconds to ~1 minute **before** the save key timestamp.

### Recommended Failure Investigation Query (Two-Step)

**Step 1 – Get the failed servicing record from CCA:**
```kql
// Run on: wdgeventstore.kusto.windows.net / CCA
UnderhillTestServicingQualityMV
| where UnderhillSvcExecutionStatus == "failed"
| extend SaveKeyTimestamp = todatetime(extract(@"^([\d\-T:.Z]+)", 1, ServicingSaveVtl2Key))
| extend StartTime = coalesce(UnderhillSvcExecutionStartTime, SaveKeyTimestamp)
| project VmId, VmName, NodeId, Cluster, StartTime,
          OldVmFirmwareIgvmVersion, NewVmFirmwareIgvmVersion,
          ServicingSaveVtl2Key
```

**Step 2 – Get error logs from UnderhillEventTable:**
```kql
// Run on: azcore.centralus.kusto.windows.net / Fa
// Replace the VmId and time range from Step 1 output
let TargetVmId = "624bc584-9f4a-44ba-b90c-58dc210a8601";
let WindowStart = datetime(2026-04-01T05:04:00Z);
let WindowEnd   = datetime(2026-04-01T05:16:00Z);
UnderhillEventTable
| where TIMESTAMP between(WindowStart .. WindowEnd)
| where VmId == TargetVmId
| where Level <= 2  // Errors and Critical only
| extend ParsedMsg = parse_json(Message)
| extend Target = tostring(ParsedMsg.Target)
| extend Fields = tostring(ParsedMsg.Fields)
| project TIMESTAMP, Level, Target, Fields
| order by TIMESTAMP asc
```

### Known Failure Root Cause (2026-04-01, VmId `624bc584...`)

**Summary:** MANA network adapter hardware was in a pre-existing failed state, causing the servicing operation to time out during VM restart.

**Complete failure chain:**

| Time (UTC) | Component | Event |
|---|---|---|
| `05:05:19.383Z` | `mana_driver::bnic_driver` | `"Previous hardware failure"` (×6 — one per NIC endpoint) |
| `05:05:19.383Z` | `mana_driver::resources` | `"failed to tear down resource"` ← `"Previous hardware failure"` (×6) |
| `05:05:19.383Z` | `mana_driver::gdma_driver` | `"Previous hardware failure"` (×2) |
| `05:05:24.559Z` | `underhill_core::worker` | `"failed to start VM"` ← `"failed to merge configuration"` ← `"cancelled waiting for mana devices"` ← `"deadline exceeded"` |
| `05:05:27.102Z` | `firmware_uefi::service::diagnostics` | UEFI DXE: `"[Bds] Unable to boot!"` |
| `05:06:07Z` | *(CCA log)* | `ServicingSaveVtl2Key` recorded (servicing attempt finalized as failed) |

**Interpretation:**
- The MANA Microsoft Azure Network Adapter (GDMA/BNIC) was already in a hardware-failed state *before* servicing began — `"Previous hardware failure"` means the HW had failed prior to this servicing attempt.
- During the save/teardown phase, the MANA driver could not clean up its 6 endpoints and 2 GDMA channels.
- On VM restart, `underhill_core` timed out (deadline exceeded) waiting for MANA devices to become ready.
- UEFI also logged `"[Bds] Unable to boot!"` — the OS couldn't boot because the network device was unavailable.
- The UEFI `PEI_CORE` and DXE errors (variable policy not found, image start failed, `AziHsmDeviceCnt:0`) are **expected/benign** noise on this platform and are **not** part of the failure chain.
- `kmsg` ITS WARNING (`"Spectre-v2 mitigation is off"`) is also benign — it indicates a VM restart was attempted (fresh kernel boot), not a causal factor.

**Key takeaway:** This failure is a **hardware node issue** (MANA NIC in bad state on node `1c202e4f-5b31...`, cluster `LVL10PrdApp56`), not a firmware or software bug. Monitoring for repeated failures on the same `NodeId` would identify degraded hardware nodes.
