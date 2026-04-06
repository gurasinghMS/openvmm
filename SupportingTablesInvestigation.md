# Supporting Kusto Tables Investigation Guide

> **Scope:** Non-HyperV / supporting Kusto tables used in Azure Host OS livesite investigations.
> Covers: MycroftContainerHealthSnapshot, HawkeyeRCAEvents, UnderhillEventTable,
> UnderhillServicingExecutionData, OsFileVersionTable, HyperVVmConfigSnapshot,
> VmHealthRawStateEtwTable, WindowsEventTable, HyperVEfiDiagnosticsTable,
> ServiceVersionSwitch, OMWorkerRepairGenerator, LogContainerSnapshot,
> Underhill\_Crashes, UnderhillVtl2OOM / UnderhillMemorySnapshotsV1, MANA tables,
> AsapNvmeEtwTraceLogEventView, SEL logs, AnyHostUpdateOnNode / nodes,
> and VmServiceEventsEtwTable.
>
> Distilled from RDOS Livesite EngHub documentation.

---

## Table of Contents

1. [MycroftContainerHealthSnapshot](#1-mycroftcontainerhealthsnapshot)
2. [HawkeyeRCAEvents](#2-hawkeyercaevents)
3. [UnderhillEventTable](#3-underhilleventtable)
4. [UnderhillServicingExecutionData](#4-underhillservicingexecutiondata)
5. [OsFileVersionTable](#5-osfileversiontable)
6. [HyperVVmConfigSnapshot](#6-hypervvmconfigsnapshot)
7. [VmHealthRawStateEtwTable](#7-vmhealthrawstateetwtable)
8. [WindowsEventTable](#8-windowseventtable)
9. [HyperVEfiDiagnosticsTable](#9-hypervefi­diagnosticstable)
10. [ServiceVersionSwitch](#10-serviceversionswitch)
11. [OMWorkerRepairGenerator](#11-omworkerrepairgenerator)
12. [LogContainerSnapshot](#12-logcontainersnapshot)
13. [Underhill\_Crashes](#13-underhill_crashes)
14. [UnderhillVtl2OOM / UnderhillMemorySnapshotsV1](#14-underhillvtl2oom--underhillmemorysnapshotsv1)
15. [MANA Tables (VNicIov, Umed, Im)](#15-mana-tables-vniciov-umed-im)
16. [AsapNvmeEtwTraceLogEventView](#16-asapnvmeetwtracelogeventview)
17. [SEL Logs (System Event Log)](#17-sel-logs-system-event-log)
18. [AnyHostUpdateOnNode / nodes](#18-anyhostupdateonnode--nodes)
19. [VmServiceEventsEtwTable](#19-vmserviceeventsetwtable)
20. [Quick-Reference: Cluster & Database Map](#20-quick-reference-cluster--database-map)
21. [Access / Security Groups Cheat-Sheet](#21-access--security-groups-cheat-sheet)

---

## 1. MycroftContainerHealthSnapshot

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus` |
| **Database** | `AzureCP` |
| **Access SG** | Included with standard RDOS Kusto access |

### What It Contains

Periodic snapshots of container (VM) health as observed by the Mycroft/AzureCP platform. Each row records the container state, lifecycle state, and fault information at a point in time. This is the primary table for understanding what Azure's control plane "thinks" about a VM.

### Key Fields

| Field | Description |
|---|---|
| `ContainerId` | The VM / container GUID |
| `NodeId` | Host node GUID |
| `Tenant` | Cluster name |
| `ContainerState` | Current state (e.g., `Started`, `Stopped`, `Faulted`) |
| `LifecycleState` | Lifecycle phase (e.g., `GoalStateAllocated`) |
| `FaultInfo` | JSON blob with fault details when the container is unhealthy |
| `PreciseTimeStamp` | Snapshot time |

### When to Use

- **First stop** for "VM has started but is unhealthy" investigations.
- Verify the most recent time a VM came up (or went down).
- Correlate container state transitions with other telemetry.

### Example Query

```kusto
// Check container health around a fault time
let fn_faultTime = datetime(2024-10-24 21:39:19.901);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
let fn_nodeId = "<nodeId>";
let fn_containerId = "<containerId>";
cluster('azcore.centralus').database('AzureCP').MycroftContainerHealthSnapshot
| where ContainerId == fn_containerId and NodeId == fn_nodeId
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| project Tenant, PreciseTimeStamp, ContainerId, ContainerState, LifecycleState, FaultInfo
| order by PreciseTimeStamp asc
```

### Tips

- If the table returns no rows, the VM may predate AzureCP migration — try `LogContainerHealthSnapshot` in AzureCM instead.
- `FaultInfo` is a JSON string; use `parse_json(FaultInfo)` to extract `.Reason` and `.FaultCode`.

---

## 2. HawkeyeRCAEvents

| Property | Value |
|---|---|
| **Cluster** | `hawkeyedataexplorer.westus2` |
| **Database** | `HawkeyeLogs` |
| **Access SG** | Standard RDOS Kusto access |

### What It Contains

Automated Root-Cause Analysis (RCA) events produced by the Hawkeye engine. Hawkeye inspects node faults and container failures, identifies known patterns (e.g., MANA init failures, workflow timeouts), and records the RCA result along with the escalation target.

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Affected host node |
| `Scenario` | Fault scenario (e.g., `ContainerUnresponsive`, `ContainerStart`, `ContainerFault`) |
| `RCALevel1` | High-level RCA category |
| `RCALevel2` | Detailed RCA sub-category |
| `EscalateToTeam` | ICM team to escalate to |
| `EscalateToOrg` | Organization for escalation |
| `FaultTime` | When the fault was observed |
| `Input` | JSON with faultInfo details |

### When to Use

- **Always check before deep-diving** into an Underhill or container fault — Hawkeye may already have an RCA.
- Aggregate `dcount(NodeId) by RCALevel2` to find fleet-wide patterns.

### Example Query

```kusto
// Check if Hawkeye already has an RCA for a specific fault
let fn_nodeId = "<nodeId>";
let fn_faultTime = datetime(2025-01-23 11:30:15);
let fn_startTime = fn_faultTime - 2d;
let fn_endTime = fn_faultTime + 2d;
cluster('hawkeyedataexplorer.westus2').database('HawkeyeLogs').HawkeyeRCAEvents
| where NodeId == fn_nodeId
| where Scenario has "ContainerUnresponsive"
    or Scenario has "ContainerStart"
    or Scenario has "ContainerFault"
| project NodeId, FaultTime, Scenario, RCALevel1, RCALevel2, EscalateToTeam, EscalateToOrg
```

```kusto
// Fleet-wide: distinct impacted nodes by failure category
cluster('hawkeyedataexplorer.westus2').database('HawkeyeLogs').HawkeyeRCAEvents
| where PreciseTimeStamp > ago(15d)
| where EscalateToTeam == "OneFleet Node\\AzureHost-Agent"
| extend fault = tostring(parse_json(Input).faultInfo)
| extend Reason = parse_json(fault).Reason
| extend FaultCode = parse_json(fault).FaultCode
| summarize dcount(NodeId) by RCALevel2
```

### Tips

- Hawkeye coverage is continuously expanding. Even if it doesn't have an RCA today, the `Input` field contains the raw fault JSON which can be useful context.
- For Underhill investigations, this is Step 2 in the Underhill TSG — check before proceeding with manual triage.

---

## 3. UnderhillEventTable

| Property | Value |
|---|---|
| **Cluster (primary)** | `wdgeventstore.kusto.windows.net` |
| **Database (primary)** | `AzureHostOs` |
| **Cluster (secondary)** | `azcore.centralus` |
| **Database (secondary)** | `Fa` |
| **Access SG** | Join **HostOsData Kusto Viewers** in idweb |

### What It Contains

Underhill kernel/runtime logs emitted via the `Microsoft.Windows.HyperV.Hcl` ETW provider. Every log message from the Underhill VTL2 environment — boot progress, device initialization, servicing events, panics, network init, etc. — appears here.

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `VmName` | Container ID / VM name |
| `Message` | JSON-structured log message |
| `Level` | ETW level (2=Error, 3=Warning, 4=Info) |
| `ProviderName` | Always `Microsoft.Windows.HyperV.Hcl` |
| `Opcode` | 1=Start, 2=Stop (useful for span tracing) |
| `CorrelationId` | Links related events (especially during servicing) |

### When to Use

- Trace Underhill boot progress and VTL2 initialization.
- Investigate panics, MANA init failures, device attach issues.
- Look up the `crate_revision` (git commit) of the running Underhill build.
- Correlate servicing events via `CorrelationId`.

### Example Query — Basic Event Lookup

```kusto
let fn_nodeId = '<nodeId>';
let fn_containerId = '<containerId>';
let fn_startTime = datetime(2025-09-15 23:50:00);
let fn_endTime = datetime(2025-09-15 23:55:00);
let fn_filter = dynamic(['vmid', 'vmname', 'fields', 'message', 'level', 'timestamp', 'op_code']);
let fn_filter2 = dynamic(['name', 'target', 'time_taken_ns', 'time_active_ns',
    'activity_id', 'related_activity_id', 'correlationid', 'correlation_id']);
cluster('wdgeventstore.kusto.windows.net').database('AzureHostOs').UnderhillEventTable
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where VmName == fn_containerId
| extend MessageParsed = parse_json(tolower(tostring(Message)))
| extend InnerMessageParsed = parse_json(tolower(tostring(MessageParsed.message)))
| extend Fields = bag_merge(MessageParsed, InnerMessageParsed)
| extend Fields = bag_remove_keys(Fields, fn_filter)
| extend Fields = bag_remove_keys(Fields, fn_filter2)
| project PreciseTimeStamp, Level, tostring(MessageParsed.target), Fields
| order by PreciseTimeStamp asc
```

### Example Query — Find Underhill Git Commit

```kusto
let fn_startTime = datetime(2024-01-17 07:35);
let fn_endTime = datetime(2024-01-17 21:35);
let fn_nodeId = "<nodeId>";
let fn_containerId = "<containerId>";
cluster('wdgeventstore.kusto.windows.net').database('AzureHostOs').UnderhillEventTable
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where VmName == fn_containerId
| extend MessageParsed = parse_json(Message)
| where MessageParsed.Target has "underhill_init"
| where MessageParsed.Message has "crate_revision"
| extend InnerMessageParsed = parse_json(tostring(MessageParsed.Message))
| parse InnerMessageParsed.fields.message with * "crate_name=" crateName ", crate_revision=" crateRevision
| project PreciseTimeStamp, ProviderName, Level, MessageParsed.Target, crateName, crateRevision
| take 5
```

### Tips

- The `Message` field is deeply nested JSON. Always `parse_json()` it and then extract sub-fields.
- Filter with `Level <= 4` to focus on errors/warnings (excludes verbose `GuestEmulationDevice::HandleRequest` noise).
- The `Fa` database on `azcore.centralus` also has an `UnderhillEventTable` — use it when `wdgeventstore` is slow or inaccessible.

---

## 4. UnderhillServicingExecutionData

| Property | Value |
|---|---|
| **Cluster** | `wdgeventstore.kusto.windows.net` |
| **Database** | `CCA` |
| **Access SG** | Join **CCA Kusto Viewer** in idweb |

### What It Contains

Detailed records of Underhill servicing (patching) operations. Each row represents a servicing execution attempt with status, timing, and version information.

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `VmName` | Container ID |
| `Operation` | Servicing operation type |
| `UnderhillSvcPkgStatus` | Result (e.g., `PATCH_SUCCESS`, `PATCH_FAILED_CANCELED`) |
| `Source` | Source component |
| `ExecutionMode` | Execution mode of the operation |
| `UnderhillSvcPackageExecutionStartTimeStamp` | Start time |
| `UnderhillSvcExecutionStartTime` | Execution start |
| `UnderhillSvcExecutionEndTime` | Execution end |
| `NewVmFirmwareIgvmVersion` | Target Underhill version |
| `EventId` | e.g., 5124 = `PATCH_FAILED_CANCELED` |

### When to Use

- Investigate servicing failures — why did a patch fail?
- Find stuck servicing operations (started but never completed).
- Correlate with `UnderhillEventTable` via `CorrelationId` for detailed logs.

### Example Query — Find Failed Servicing

```kusto
let startTime = datetime(2024-09-01);
let endTime = now();
let fn_Operation = "HotPatch";
cluster('wdgeventstore.kusto.windows.net').database('CCA').UnderhillServicingExecutionData
| where UnderhillSvcPackageExecutionStartTimeStamp between (startTime .. endTime)
| where Operation == fn_Operation
| where UnderhillSvcPkgStatus != "PATCH_SUCCESS"
| where Source startswith "UnderhillSvc"
| where ExecutionMode == "Normal"
| project NodeId, VmName, Operation, UnderhillSvcPkgStatus, NewVmFirmwareIgvmVersion,
    UnderhillSvcExecutionStartTime, UnderhillSvcExecutionEndTime
```

### Example Query — Map DLL Version to Git Commit

```kusto
// Given a DLL version, find the corresponding git commit hash
cluster('wdgeventstore.kusto.windows.net').database('CCA').GetUnderhillBinaryCommitHash('1.2.98.0')
| take 3
```

### Tips

- Combine with `UnderhillEventTable` by matching on `CorrelationId` to get full servicing logs.
- For "stuck" servicing, look for entries entering `servicing_save_vtl2` (Opcode=1) with no matching exit (Opcode=2).

---

## 5. OsFileVersionTable

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus` |
| **Database** | `Fa` |
| **Access SG** | Standard RDOS Kusto access |

### What It Contains

Periodic snapshots of binary file versions on the host. The agent logs file metadata (name, version, timestamp, size) for key OS binaries. Logging happens daily and upon node updates.

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `FileName` | Binary name (e.g., `vmfirmwareigvm.dll`) |
| `FileVersion` | Version string |
| `FileTimeStamp` | File's build timestamp |
| `FileSize` | File size in bytes |

### When to Use

- **Determine the Underhill version** by querying for `vmfirmwareigvm.dll`.
- Verify specific binary versions deployed on a node.
- Check if a hotfix binary was applied.

### Example Query

```kusto
let fn_startTime = datetime(2024-11-02 07:35);
let fn_endTime = datetime(2024-11-02 21:35);
let fn_nodeId = "<nodeId>";
cluster('azcore.centralus').database('Fa').OsFileVersionTable
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where FileName == "vmfirmwareigvm.dll"
| where FileVersion != "FileNotFound"
| project PreciseTimeStamp, FileName, FileVersion, FileTimeStamp, FileSize
```

### Tips

- When a node is patch-updated, the file version logged may remain the same if that specific binary didn't change — multiple rows with the same version is expected.
- Logging is periodic (daily) **and** triggered by node updates, not only when the specific file changes.
- For HCL version, query `FileName == "vmfirmwarehcl.dll"`.

---

## 6. HyperVVmConfigSnapshot

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus` |
| **Database** | `Fa` |
| **Access SG** | Standard RDOS Kusto access |

### What It Contains

Periodic snapshots of VM configuration details including Underhill settings, memory allocation, firmware file info, and device config. **Only populated on hosts running AH2023 and newer.**

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `ContainerId` | Container / VM ID |
| `SummaryType` | e.g., `Configuration` |
| `SummaryJson` | Full JSON with all config details |
| `IsUnderhill` | Whether the VM is an Underhill VM |
| `VmId` | Virtual Machine unique ID |
| `Cluster` | Cluster name |
| `Region` | Azure region |

### When to Use

- **Step 1 of Underhill TSG**: Determine if a VM is actually an Underhill VM.
- Find VTL2 memory allocation (`Vtl2RamBaseAddrOffsetMb`).
- Get current firmware file/version from `ManagementVtlState`.
- Enumerate Underhill clusters fleet-wide.

### Example Query — Check if VM is Underhill

```kusto
let fn_nodeId = "<nodeId>";
let fn_containerId = "<containerId>";
let fn_faultTime = datetime(2024-04-24T02:33:08Z);
let fn_startTime = fn_faultTime - 1d;
let fn_endTime = fn_faultTime + 1h;
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot
| where NodeId == fn_nodeId and ContainerId == fn_containerId
    and PreciseTimeStamp between(fn_startTime .. fn_endTime)
| where SummaryType == "Configuration"
| extend IsUnderhillFromJson = parse_json(SummaryJson).Settings.hcl.IsUnderhill
| project PreciseTimeStamp, IsUnderhill = iff(isnotempty(IsUnderhill), IsUnderhill, IsUnderhillFromJson)
| order by PreciseTimeStamp desc
| take 1
```

### Example Query — Get Firmware Version from Config

```kusto
let fn_nodeId = "<nodeId>";
let fn_containerId = "<containerId>";
let fn_startTime = datetime(2024-09-01);
let fn_endTime = datetime(2024-09-02);
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId and ContainerId == fn_containerId
| where SummaryJson contains "vmfirmwareigvm"
| extend m = parse_json(SummaryJson)
| extend vtl = parse_json(m.ManagementVtlState)
| extend state = parse_json(m.VmState)
| project state.Current, vtl.CurrentFileName, vtl.CurrentFileVersion
```

### Tips

- **AH2023+ only.** Older host OSes don't log to this table. Fall back to `OsFileVersionTable` or ask the VMService team.
- Use `SummaryJson contains "vmfirmwareigvm"` to filter to rows that have firmware info.
- For fleet-wide Underhill cluster enumeration: `| where IsUnderhill == 'true' | summarize dcount(NodeId) by Cluster`.

---

## 7. VmHealthRawStateEtwTable

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus` |
| **Database** | `Fa` |
| **Access SG** | Standard RDOS Kusto access |

### What It Contains

Real-time VM health state changes as observed by the Host Agent. Records transitions in heartbeat, power state, VSC state, and handshake completion — the signals that determine whether a VM is "healthy" or "unhealthy."

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `ContainerId` | Container / VM ID |
| `VmIncarnationId` | VM incarnation (changes on restart) |
| `VmHyperVIcHeartbeat` | Integration Component heartbeat state |
| `VmPowerState` | Power state (Running, Off, etc.) |
| `HasHyperVHandshakeCompleted` | Whether IC handshake completed |
| `IsVscStateOperational` | Whether Virtual Service Clients are operational |
| `Context` | Additional context JSON |
| `VirtualMachineUniqueId` | Stable VM identifier |

### When to Use

- Investigate "VM is started but unhealthy" scenarios.
- Track the exact moment a VM transitioned from healthy to unhealthy (or vice versa).
- Understand which health signal failed (heartbeat, VSC, handshake).

### Example Query

```kusto
let fn_startTime = datetime(2024-10-24 20:39:19);
let fn_endTime = datetime(2024-10-24 22:39:19);
let fn_containerId = "<containerId>";
cluster('azcore.centralus').database('Fa').VmHealthRawStateEtwTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where ContainerId == fn_containerId
| project PreciseTimeStamp, ContainerId, VmHyperVIcHeartbeat, VmPowerState,
    HasHyperVHandshakeCompleted, IsVscStateOperational, Context
| sort by PreciseTimeStamp asc
| extend PrevTime = prev(PreciseTimeStamp)
| extend NextTime = next(PreciseTimeStamp)
```

### Tips

- The table logs **state changes**, not periodic snapshots. Gaps between rows mean the state was stable.
- A VM is considered "unhealthy" when `VmHyperVIcHeartbeat` is not `OK` or `IsVscStateOperational` is `false`.
- Cross-reference with `MycroftContainerHealthSnapshot` to see how the control plane interprets these signals.

---

## 8. WindowsEventTable

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus` |
| **Database** | `Fa` |
| **Access SG** | Standard RDOS Kusto access |

### What It Contains

Windows Event Log entries captured from the host. Includes Hyper-V worker events, chipset events, VM lifecycle events, OS errors, and more. This is the Kusto equivalent of the Windows Event Viewer.

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `ProviderName` | ETW provider (e.g., `Microsoft-Windows-Hyper-V-Worker`, `Microsoft-Windows-Hyper-V-Chipset`) |
| `EventId` | Event identifier |
| `Description` | Human-readable event description |
| `Level` | Severity level |
| `PreciseTimeStamp` | Event time |

### When to Use

- Check for UEFI boot diagnostics (EventId 18600 from Hyper-V-Chipset).
- Detect guest crashes (EventId 18603/18604 — crash dump success/failure).
- Find VM start events (EventId 18500), reset events (EventId 18514).
- Investigate HCL/firmware faults (EventId 18610 from Hyper-V-Worker).
- Look for "fatal virtual firmware error" events.

### Example Query — EFI Diagnostics via Chipset Events

```kusto
let fn_nodeId = "<nodeId>";
let fn_containerId = "<containerId>";
let fn_vmId = "<vmUniqueId>";
let fn_startTime = datetime(2024-10-07 01:00);
let fn_endTime = datetime(2024-10-07 04:00);
cluster('azcore.centralus').database('Fa').WindowsEventTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    and EventId == 18600
    and ProviderName == "Microsoft-Windows-Hyper-V-Chipset"
    and (Description has fn_containerId or Description has fn_vmId)
| project PreciseTimeStamp, NodeId, Description, Level
```

### Example Query — Guest Lifecycle Events

```kusto
let fn_nodeId = "<nodeId>";
let fn_containerId = "<containerId>";
let fn_vmId = "<vmUniqueId>";
let fn_startTime = datetime(2024-10-01);
let fn_endTime = datetime(2024-10-02);
let fn_guestErrors = dynamic([
    18500, // MSVM_VM_STARTED
    18514, // MSVM_GUEST_RESET_SUCCESS
    18601, // MSVM_GUEST_CRASH_NOTIFICATION
    18603, // MSVM_GUEST_CRASH_DUMP_SUCCESS
    18604  // MSVM_GUEST_CRASH_DUMP_FAILURE
]);
cluster('azcore.centralus').database('Fa').WindowsEventTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    and ProviderName == "Microsoft-Windows-Hyper-V-Worker"
    and EventId in (fn_guestErrors)
| where (Description has fn_containerId or Description has fn_vmId)
| project PreciseTimeStamp, NodeId, Description, Level, EventId
```

### Tips

- The table name is `WindowsEventTable` (not `WindowsEventsTable` — no 's').
- For Gen2 VMs, POST code / boot diagnostics come from `Microsoft-Windows-Hyper-V-Chipset` EventId 18600.
- Always filter by `ProviderName` and `EventId` to avoid scanning the entire table.

---

## 9. HyperVEfiDiagnosticsTable

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus` |
| **Database** | `Fa` |
| **Access SG** | Standard RDOS Kusto access |

### What It Contains

UEFI/EFI boot diagnostics data for Generation 2 VMs. Records POST codes, boot phase durations, firmware initialization events, and boot failure details. This table captures the firmware-level view of VM boot progress.

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `ContainerId` | VM container ID |
| `VmId` | Virtual Machine unique ID |
| `PostCode` | UEFI POST code |
| `BootPhase` | Current boot phase |
| `BootDurationMs` | Duration of boot phase in milliseconds |
| `DiagnosticData` | Additional firmware diagnostic data |
| `PreciseTimeStamp` | Event time |

### When to Use

- VM fails to boot — check what POST code it got stuck on.
- Measure boot times and identify slow boot phases.
- Compare boot diagnostics before and after a firmware update.

### Example Query

```kusto
let fn_nodeId = "<nodeId>";
let fn_containerId = "<containerId>";
let fn_startTime = datetime(2024-10-01);
let fn_endTime = datetime(2024-10-02);
cluster('azcore.centralus').database('Fa').HyperVEfiDiagnosticsTable
| where NodeId == fn_nodeId
| where ContainerId == fn_containerId
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| project PreciseTimeStamp, ContainerId, PostCode, BootPhase, BootDurationMs, DiagnosticData
| order by PreciseTimeStamp asc
```

### Tips

- This is only relevant for **Generation 2 VMs** (UEFI-based). Gen1 VMs use legacy BIOS and don't emit EFI diagnostics.
- Also check `WindowsEventTable` with `ProviderName == "Microsoft-Windows-Hyper-V-Chipset"` and `EventId == 18600` for additional boot diagnostics.
- For Underhill VMs, the VTL2 boot path adds extra POST codes not seen in standard Gen2 VMs.

---

## 10. ServiceVersionSwitch

| Property | Value |
|---|---|
| **Cluster** | `azdeployer` |
| **Database** | `AzDeployerKusto` |
| **Access SG** | Join **AzDeployer Kusto User SG** in idweb |

### What It Contains

Records of deployment version changes — when a service version was switched on a cluster or set of nodes. Used to track which version of the Host OS (or other components) is deployed where.

### Key Fields

| Field | Description |
|---|---|
| `ClusterName` | Fabric cluster |
| `Environment` | Deployment environment |
| `ServiceName` | The service being deployed |
| `OldVersion` | Previous version |
| `NewVersion` | New version |
| `SwitchTime` | When the switch occurred |
| `SdpPhase` | SDP rollout phase (Canary, Pilot, Broadphase, etc.) |
| `Region` | Azure region |

### When to Use

- Determine when a particular version was deployed to a cluster.
- Investigate whether a recent version switch correlates with new faults.
- Track rollout progress across SDP phases.

### Example Query

```kusto
cluster('azdeployer').database('AzDeployerKusto').ServiceVersionSwitch
| where PreciseTimeStamp > ago(7d)
| where ClusterName == "<clusterName>"
| project PreciseTimeStamp, ClusterName, ServiceName, OldVersion, NewVersion, SdpPhase, Region
| order by PreciseTimeStamp desc
```

### Tips

- Combine with `DeploymentAuditEvent` for a more complete deployment picture.
- Use the `StagerTargetStatusLog` table to check what SDP stage a cluster is currently in.

---

## 11. OMWorkerRepairGenerator

| Property | Value |
|---|---|
| **Cluster** | `azdeployer` |
| **Database** | `AzDeployerKusto` |
| **Access SG** | Join **AzDeployer Kusto User SG** in idweb |

### What It Contains

Records of repair actions initiated by the Orchestration Manager (OM) worker. When a node enters a fault state (e.g., `HumanInvestigate`, `OutForRepair`), the OM system may generate repair actions such as reboots, reimages, or hardware replacement requests.

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Target node |
| `ClusterName` | Cluster |
| `RepairAction` | Type of repair (e.g., `Reboot`, `Reimage`, `HardwareReplace`) |
| `RepairReason` | Why the repair was initiated |
| `RepairState` | Current state of the repair |
| `RequestTime` | When the repair was requested |
| `CompletionTime` | When the repair completed |

### When to Use

- Check if a repair was already attempted on a problem node.
- Understand the repair history for a node experiencing repeated failures.
- Verify if a reimage or hardware replacement is in progress.

### Example Query

```kusto
cluster('azdeployer').database('AzDeployerKusto').OMWorkerRepairGenerator
| where PreciseTimeStamp > ago(14d)
| where NodeId == "<nodeId>"
| project PreciseTimeStamp, NodeId, ClusterName, RepairAction, RepairReason, RepairState
| order by PreciseTimeStamp desc
```

### Tips

- Cross-reference with `LogNodeSnapshot` in AzureCM to see the node's state history.
- Nodes stuck in `HumanInvestigate` often have pending repair actions visible here.

---

## 12. LogContainerSnapshot

| Property | Value |
|---|---|
| **Cluster** | `azurecm` (also aliased as `gandalf`) |
| **Database** | `AzureCM` |
| **Access SG** | Standard RDOS Kusto access |

### What It Contains

Container Manager's view of all containers (VMs) in the fleet. Includes container properties, subscription info, VM configuration, billing type, and network/security settings. This is the CM-authoritative source for "what is this VM?"

### Key Fields

| Field | Description |
|---|---|
| `containerId` | Container / VM ID |
| `nodeId` | Host node |
| `Tenant` | Cluster name |
| `roleInstanceName` | Azure VM computer name |
| `subscriptionId` | Customer subscription |
| `virtualMachineUniqueId` | Stable VM identifier |
| `additionalContainerProperties` | JSON with security, colocation, billing settings |
| `tipNodeSessionId` | TiP session ID (if part of a test) |
| `billingType` | Windows/Linux billing classification |

### When to Use

- Look up VM metadata: subscription, VM unique ID, role instance name.
- Check container security properties (SecureBoot, vTPM, CVM, TVM).
- Determine VM generation (check `additionalContainerProperties`).
- Find VMs from a computer name or subscription ID.

### Example Query — Find VM from Computer Name

```kusto
let fn_subscriptionId = '<subscriptionId>';
let fn_roleInstanceName = "s-np-590950d1";
cluster('azurecm').database('AzureCM').LogContainerSnapshot
| where TIMESTAMP >= ago(7d)
| where subscriptionId == fn_subscriptionId
| where roleInstanceName has fn_roleInstanceName
| distinct nodeId, containerId, virtualMachineUniqueId, subscriptionId,
    roleInstanceName, Tenant, tipNodeSessionId
```

### Example Query — Check Container Properties

```kusto
let fn_containerId = "<containerId>";
cluster('azurecm').database('AzureCM').LogContainerSnapshot
| where containerId == fn_containerId
| where PreciseTimeStamp > ago(7d)
| project PreciseTimeStamp, containerId, nodeId, Tenant, additionalContainerProperties
| take 1
```

### Tips

- `AzureCM` by default uses weak consistency and has huge metadata. If Kusto Desktop times out, add `set queryconsistency = 'strongconsistency';` at the top.
- `LogContainerHealthSnapshot` (same cluster/database) provides the health view; `LogContainerSnapshot` provides the metadata view.
- When `LogContainerSnapshot` is missing data, try `MycroftContainerSnapshot` on `mycroft.westcentralus / Mycroft`.

---

## 13. Underhill\_Crashes

| Property | Value |
|---|---|
| **Cluster** | `hostosdata.centralus` |
| **Database** | `NFP` |
| **Access SG** | Join **HostOsData Kusto Viewers** in idweb |

### What It Contains

Crash dump metadata for Underhill VM panics and bugchecks. When an Underhill VM panics, the crash is bucketed by its panic message / stack and recorded here. The Theseus automation monitors this table and auto-creates IcM incidents for new bugcheck buckets.

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Host node where crash occurred |
| `Cluster` | Cluster name |
| `TipLabel` | Whether the node was a TiP node |
| `bucketString` | Crash bucket identifier (panic message or stack hash) |
| `bucketURL` | Link to crash analysis details |
| `crashProcessFullPath` | Path of the crashing process |
| `dumpUid` | Unique dump identifier |
| `dumpURL` | URL to access the dump file |
| `PreciseTimeStamp` | Time of crash |

### When to Use

- Investigate Underhill panics / VTL2 crashes.
- Check if a crash is already being tracked by the Theseus automation.
- Assess fleet-wide impact of a specific crash bucket.

### Example Query — Fleet-Wide Crash Buckets

```kusto
cluster('hostosdata.centralus').database('NFP').Underhill_Crashes
| where PreciseTimeStamp > ago(15d)
| where TipLabel == "NonTipNodes"
| summarize TotalNodes = dcount(NodeId), TotalHits = count()
    by bucketString, bucketURL
| where TotalNodes > 3
| sort by TotalNodes desc
```

### Example Query — Crashes for a Specific Node

```kusto
cluster('hostosdata.centralus').database('NFP').Underhill_Crashes
| where PreciseTimeStamp > ago(30d)
| where NodeId == "<nodeId>"
| project PreciseTimeStamp, crashProcessFullPath, NodeId, Cluster, TipLabel, dumpUid, dumpURL, bucketString
```

### Tips

- The Theseus automation auto-creates IcMs for new crash buckets — always check ICM before filing a new one.
- Cross-reference `bucketString` with the panic message from `WindowsEventTable` EventId 18590.
- Filtering `TipLabel == "NonTipNodes"` excludes test/TiP nodes for accurate fleet impact assessment.

---

## 14. UnderhillVtl2OOM / UnderhillMemorySnapshotsV1

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus` |
| **Database** | `SharedWorkspace` |
| **Access SG** | Standard RDOS Kusto access |

### What It Contains

Memory diagnostic data for Underhill VTL2 out-of-memory (OOM) events. `UnderhillMemorySnapshotsV1` provides periodic memory snapshots, while `UnderhillVtl2OOM` records actual OOM events with memory allocation state at the time of failure.

### Key Fields — UnderhillMemorySnapshotsV1

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `ContainerId` | VM container ID |
| `TotalMemoryMb` | Total VTL2 memory allocated |
| `FreeMemoryMb` | Free memory at snapshot time |
| `MemoryConsumers` | Breakdown of memory usage by component |

### Key Fields — UnderhillVtl2OOM

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `ContainerId` | VM container ID |
| `OomTimestamp` | When OOM occurred |
| `AllocationSize` | Size of the failed allocation |
| `MemoryState` | Memory state at OOM time |

### When to Use

- Investigate Underhill VTL2 OOM events.
- Trend VTL2 memory consumption over time.
- Identify memory-hungry components in the Underhill environment.
- Verify VTL2 memory configuration from `HyperVVmConfigSnapshot` against actual usage.

### Example Query — Check VTL2 Memory Configuration

```kusto
// First check configured memory via HyperVVmConfigSnapshot
let fn_nodeId = "<nodeId>";
let fn_containerId = "<containerId>";
let fn_startTime = datetime(2024-10-01);
let fn_endTime = datetime(2024-10-02);
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId and ContainerId == fn_containerId
| where SummaryJson contains "Vtl2RamBaseAddrOffsetMb"
| extend m = parse_json(SummaryJson)
| extend mem = parse_json(m.Memory)
| project mem
```

### Example Query — Check Initial VTL2 Settings from UnderhillEventTable

```kusto
let fn_nodeId = "<nodeId>";
let fn_containerId = "<containerId>";
let fn_faultTime = datetime(2024-10-01);
cluster('azcore.centralus').database('Fa').UnderhillEventTable
| where NodeId == fn_nodeId and VmName == fn_containerId
| where TIMESTAMP <= fn_faultTime
| where Message has "Initial VTL2 settings"
| extend InternalMessage = parse_json(tostring(parse_json(Message).Message))
| parse InternalMessage with * "Vtl2SettingsFile" rest
| project PreciseTimeStamp, rest
| take 1
```

### Tips

- The `SharedWorkspace` database also contains helper functions like `AgentOperations()` and `HyperVContainerStarted()` that are useful for broader investigations.
- VTL2 memory is carved from the VM's total memory allocation — OOM issues can indicate the VM's VTL2 partition is undersized for its workload.

---

## 15. MANA Tables (VNicIov, Umed, Im)

| Property | Value |
|---|---|
| **Cluster** | `netperf` |
| **Database** | `NetPerfKustoDB` |
| **Access SG** | Check with Azure Networking team for access |

### What It Contains

Microsoft Azure Network Adapter (MANA) telemetry tables. These track the SR-IOV virtual NIC (VNicIov), the underlying MANA device (Umed), and the interrupt moderation (Im) configuration. Used for networking performance and failure investigations.

### Key Fields (Representative — VNicIov)

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `VmId` | VM identifier |
| `VnicId` | Virtual NIC identifier |
| `TxPackets` / `RxPackets` | Packet counters |
| `TxDrops` / `RxDrops` | Dropped packet counters |
| `LinkState` | NIC link state |

### When to Use

- Investigate MANA init failures (one of the most common Underhill IcM causes).
- Diagnose networking performance issues.
- Check for dropped packets or link state changes.

### SoC MANA Logs (Alternative Path)

For SoC-based MANA logs (Overlake), use the `LinuxOverlakeSystemd` table:

```kusto
let fn_NodeId = "<nodeId>";
let fn_StartTime = datetime(2024-02-24);
let fn_EndTime = datetime(2024-02-25);
let socID = toscalar(
    cluster('azuredcm.kusto.windows.net').database('AzureDCMDb')
    .GetSocOrNodeFromResourceId(fn_NodeId)
);
cluster('azcore.centralus.kusto.windows.net').database('OvlProd').LinuxOverlakeSystemd
| where NodeId =~ fn_NodeId or NodeId =~ socID
| where PreciseTimeStamp between (fn_StartTime .. fn_EndTime)
| where _SYSTEMD_UNIT startswith "socmana"
    or _SYSTEMD_UNIT startswith "gdma-vfio"
    or _SYSTEMD_UNIT startswith "soc-mana-boot"
| project PreciseTimeStamp, _SYSTEMD_UNIT, _PID, MESSAGE
| order by PreciseTimeStamp asc
```

### Tips

- The SoC MANA version can be found in the MESSAGE field: `[bnic v=a835a9 h=0316]` — the `v` value corresponds to a git commit in the SmartNIC-SW-GDMA repo.
- MANA init failures are auto-detected by Hawkeye — always check `HawkeyeRCAEvents` first.
- The `shutdown_mana` operation in `UnderhillEventTable` is critical during servicing — stuck MANA shutdown is a known failure mode.

---

## 16. AsapNvmeEtwTraceLogEventView

| Property | Value |
|---|---|
| **Cluster** | `storageclient.eastus` |
| **Database** | `Fa` |
| **Access SG** | Check with storage team for access |

### What It Contains

Azure Storage Accelerated Plane (ASAP) NVMe traces. Records NVMe device operations, errors, and performance data for VMs using NVMe storage pass-through (common in Underhill configurations).

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `PreciseTimeStamp` | Event time |
| `TraceMessage` | NVMe trace log content |
| `Level` | Severity |

### When to Use

- Investigate NVMe/storage-related Underhill start failures.
- Diagnose ASAP device errors during VM boot.
- Check for storage path issues in Underhill VTL2 initialization.

### Example Query

```kusto
let startTime = datetime(2025-04-29T10:26:59Z);
let endTime = datetime(2025-04-29T10:46:59Z);
let nodeid = "<nodeId>";
union cluster('storageclient.eastus.kusto.windows.net').database('Fa').AsapNvmeEtwTraceLogEventView
| where NodeId == nodeid
| where PreciseTimeStamp between (startTime .. endTime)
```

### Tips

- This table is referenced in the Underhill TSG when investigating VTL2 start failures — specifically in the "Check ASAP Logs" step.
- Underhill VMs commonly use NVMe through ASAP for storage — errors here can cause VM start failures.
- The table is in a different cluster (`storageclient.eastus`) than most other Host OS tables.

---

## 17. SEL Logs (System Event Log)

| Property | Value |
|---|---|
| **Cluster (interpreted)** | `baseplatform.<region>` |
| **Database (interpreted)** | `vmphu` |
| **Cluster (raw)** | `sparkle.<region>` |
| **Database (raw)** | `defaultdb` |
| **Access SG** | Join **VMPHU Kusto Viewer SG** and **SparkleUsers SG** in idweb |

### What It Contains

Baseboard Management Controller (BMC) System Event Logs. The SEL is written by hardware-level firmware and survives OS crashes/reboots, making it invaluable for diagnosing hardware-level issues, boot failures, and bugchecks.

Event types include:
- **Boot events**: OS boot timestamps and cold/soft boot indicators
- **Shutdown events**: Shutdown reason codes and optional messages
- **BSOD events**: Bugcheck codes and all 4 bugcheck parameters
- **Boot environment events**: BootMgr/Winload checkpoints and errors
- **OSREC events**: OS recovery records (WHEA errors, driver records)

### Key Fields (Interpreted Table via `RdosSelByNodeId`)

| Field | Description |
|---|---|
| `IngestionTimestamp` | When the record was ingested |
| `EstimatedRecordTimestamp` | Estimated time of the SEL event |
| `RecordDescription` | Human-readable event description |
| `EventDetail` | Decoded event details |
| `RawHex` | Raw hexadecimal SEL record |

### When to Use

- **Node does not boot** — check for boot checkpoint events and failure codes.
- **Node bugchecked** — retrieve bugcheck code and parameters from SEL BSOD events.
- **Unexplained reboots** — check shutdown events and OSREC records.

### Example Query — Interpreted SEL (Recommended)

```kusto
let fn_node = '<nodeId>';
let fn_eventtime = datetime(2024-07-01 01:01:17);
let startTime = fn_eventtime - 1d;
let endTime = fn_eventtime + 1h;
cluster('baseplatform.<region>.kusto.windows.net').database('vmphu')
    .RdosSelByNodeId(fn_node, startTime, endTime)
| project IngestionTimestamp, EstimatedRecordTimestamp, RecordDescription, EventDetail, RawHex
```

### Example Query — Raw SEL

```kusto
let fn_node = pack_array('<nodeId>');
let fn_eventtime = datetime(2024-01-30 17:17:03);
let startTime = fn_eventtime - 1d;
let endTime = fn_eventtime + 1h;
cluster('sparkle.<region>.kusto.windows.net').database('defaultdb')
    .SparkleSELByNodeIds(fn_node, startTime, endTime)
```

### Tips

- **Always try the interpreted table first** (`RdosSelByNodeId`) as it decodes OEM events automatically.
- Replace `<region>` with the appropriate Azure region (e.g., `westus`, `eastus2`).
- SEL is **not available** on HP VHM (Very High Memory) SKUs. Check: `cluster('wdgeventstore').database('HostOSDeploy').nodes | where OEM == "HP" and SKU contains "VHM"`.
- SEL logs can also be viewed on Node Story, or collected manually via `ipmiutil sel` with JIT PlatformAdministrator access.
- SEL events may have partial data — BMC writes are best-effort and not guaranteed.

---

## 18. AnyHostUpdateOnNode / nodes

| Property | Value |
|---|---|
| **Cluster** | `wdgeventstore.kusto.windows.net` |
| **Database** | `HostOSDeploy` |
| **Access SG** | Standard RDOS Kusto access |

### What It Contains

Deployment tracking data. `AnyHostUpdateOnNode` tracks when Host OS updates were applied to individual nodes. The `nodes` table provides node metadata including OEM, SKU, hardware generation, and cluster membership.

### Key Fields — AnyHostUpdateOnNode

| Field | Description |
|---|---|
| `NodeId` | Target node |
| `UpdateVersion` | Version of the update applied |
| `UpdateType` | Type of update (HotPatch, ColdPatch, etc.) |
| `UpdateTime` | When the update was applied |
| `Status` | Success/failure status |

### Key Fields — nodes

| Field | Description |
|---|---|
| `nodeId` | Node GUID |
| `OEM` | Hardware manufacturer (HP, Dell, etc.) |
| `SKU` | Hardware SKU |
| `Cluster` | Cluster assignment |
| `Generation` | Hardware generation |

### When to Use

- Track what updates have been applied to a node.
- Verify whether a specific patch reached a node.
- Look up hardware metadata (OEM, SKU, generation) for a node.
- Identify VHM nodes that don't support certain features (e.g., SEL).

### Example Query — Node Metadata

```kusto
cluster('wdgeventstore.kusto.windows.net').database('HostOSDeploy').nodes
| where nodeId == "<nodeId>"
| project nodeId, OEM, SKU, Cluster, Generation
```

### Example Query — Update History

```kusto
cluster('wdgeventstore.kusto.windows.net').database('HostOSDeploy').AnyHostUpdateOnNode
| where NodeId == "<nodeId>"
| where PreciseTimeStamp > ago(30d)
| project PreciseTimeStamp, NodeId, UpdateVersion, UpdateType, Status
| order by PreciseTimeStamp desc
```

### Tips

- The `nodes` table is useful for filtering out VHM nodes when checking SEL availability.
- Combine with `ServiceVersionSwitch` to correlate fleet-level deployments with node-level update application.

---

## 19. VmServiceEventsEtwTable

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus` |
| **Database** | `Fa` |
| **Access SG** | Standard RDOS Kusto access |

### What It Contains

Events from the VM Service (VmService / Vmal) component on the host. Records VM lifecycle operations, container management actions, configuration changes, and internal service events. This is the "VMService perspective" of container operations.

### Key Fields

| Field | Description |
|---|---|
| `NodeId` | Host node |
| `ContainerId` | VM container ID |
| `Message` | Event message |
| `Level` | Severity |
| `EventId` | Event identifier |
| `OperationName` | VM operation being performed |
| `PreciseTimeStamp` | Event time |

### When to Use

- Investigate VM start/stop/restart operations from the VMService perspective.
- Diagnose container creation or configuration failures.
- Trace the VM lifecycle through the service layer.

### Example Query

```kusto
let fn_nodeId = "<nodeId>";
let fn_containerId = "<containerId>";
let fn_startTime = datetime(2024-10-01);
let fn_endTime = datetime(2024-10-02);
cluster('azcore.centralus').database('Fa').VmServiceEventsEtwTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where Message has fn_containerId
| project PreciseTimeStamp, NodeId, EventId, OperationName, Message, Level
| order by PreciseTimeStamp asc
```

### Tips

- This table works alongside `VmServiceContainerOperation` (operation-level events) and `NodeServiceEventEtwTable` (Node Service perspective).
- For a combined view, use `VmServiceContainerOperations` which has an `IsUnderhillLocalEnabled` flag for quick Underhill filtering.

---

## 20. Quick-Reference: Cluster & Database Map

| Table | Cluster | Database |
|---|---|---|
| MycroftContainerHealthSnapshot | `azcore.centralus` | `AzureCP` |
| HawkeyeRCAEvents | `hawkeyedataexplorer.westus2` | `HawkeyeLogs` |
| UnderhillEventTable | `wdgeventstore` | `AzureHostOs` (also `azcore.centralus` / `Fa`) |
| UnderhillServicingExecutionData | `wdgeventstore` | `CCA` |
| OsFileVersionTable | `azcore.centralus` | `Fa` |
| HyperVVmConfigSnapshot | `azcore.centralus` | `Fa` |
| VmHealthRawStateEtwTable | `azcore.centralus` | `Fa` |
| WindowsEventTable | `azcore.centralus` | `Fa` |
| HyperVEfiDiagnosticsTable | `azcore.centralus` | `Fa` |
| ServiceVersionSwitch | `azdeployer` | `AzDeployerKusto` |
| OMWorkerRepairGenerator | `azdeployer` | `AzDeployerKusto` |
| LogContainerSnapshot | `azurecm` | `AzureCM` |
| Underhill\_Crashes | `hostosdata.centralus` | `NFP` |
| UnderhillVtl2OOM / UnderhillMemorySnapshotsV1 | `azcore.centralus` | `SharedWorkspace` |
| MANA tables (VNicIov, Umed, Im) | `netperf` | `NetPerfKustoDB` |
| AsapNvmeEtwTraceLogEventView | `storageclient.eastus` | `Fa` |
| SEL (interpreted) | `baseplatform.<region>` | `vmphu` |
| SEL (raw) | `sparkle.<region>` | `defaultdb` |
| AnyHostUpdateOnNode / nodes | `wdgeventstore` | `HostOSDeploy` |
| VmServiceEventsEtwTable | `azcore.centralus` | `Fa` |

---

## 21. Access / Security Groups Cheat-Sheet

To query these tables you need the appropriate Kusto access. Request membership via **idweb** or **CoreIdentity**:

| Security Group | Grants Access To |
|---|---|
| **AzDeployer Kusto User SG** | `azdeployer` — ServiceVersionSwitch, OMWorkerRepairGenerator, DeploymentAuditEvent |
| **VMPHU Kusto Viewer SG** | `baseplatform.<region>` — SEL interpreted tables |
| **SparkleUsers SG** | `sparkle.<region>` — SEL raw tables |
| **HostOsData Kusto Viewers SG** | `hostosdata.centralus` — Underhill\_Crashes, OverlakeClusterVersions |
| **CCA Kusto Viewer** | `wdgeventstore / CCA` — UnderhillServicingExecutionData |
| **IcM-Kusto-Access** | `icmcluster` — IcM incident queries |
| **XLivesiteKustoAccess** | `xstore.kusto.windows.net` — Host Analyzer |
| **AlbusViewer SG** | `gandalffollower.centralus` — Albus node fault monitoring |

> **Tip:** Use the [crowd-sourced map of Kusto clusters](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/tsg-overview) to find which SG grants access to a specific cluster. The **Check Access** section in the TSG overview provides a one-stop verification tool.

---

*Document distilled from RDOS Livesite EngHub documentation. For the latest information, see the [RDOS TSG Overview](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/tsg-overview) and the [Underhill Kusto Queries FAQ](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-kusto-queries-faq).*
