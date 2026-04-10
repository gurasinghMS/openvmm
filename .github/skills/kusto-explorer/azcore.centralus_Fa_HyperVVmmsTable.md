# HyperVVmmsTable

**Type:** Table  
**Cluster:** `https://azcore.centralus.kusto.windows.net`  
**Database:** `Fa`  
**Full Path:** `azcore.centralus.kusto.windows.net` → `Fa` → `HyperVVmmsTable`

---

## 1. Description

`HyperVVmmsTable` contains events and traces from Hyper-V's **Virtual Machine Management Service (`vmms.exe`)** running on Azure host nodes. VMMS is the core management service that handles VM lifecycle operations — creating, configuring, starting, stopping, migrating, and deleting virtual machines. This table records operational events, diagnostic traces, errors, and WMI interactions from the VMMS process across the Azure fleet.

> **Note:** This table has some noisy events **pre-filtered out** at ingestion. For combined queries you should also manually exclude `"WHERE clause operator"` and `"Provider could not handle query"` spam.

> **Sources:**
> - [Hyper-V Kusto Queries](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/hyperv-kusto-queries)
> - [Stop Container Failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/stop-container-failure)
> - [Migration Failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/migration/migration-failure)
> - [Underhill Servicing TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-servicing)
> - [Underhill Kusto Queries FAQ](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-kusto-queries-faq)

**Data volume:** ~148 billion rows per day (extremely high volume).

**Level distribution (last 24h):**
| Level | Meaning  | Count (~)         | Percentage |
|-------|----------|-------------------|------------|
| 1     | Critical | 5                 | ~0%        |
| 2     | Error    | 5.7 billion       | ~3.8%      |
| 3     | Warning  | 74.5 billion      | ~50.3%     |
| 4     | Info     | 21.3 billion      | ~14.3%     |
| 5     | Verbose  | 46.8 billion      | ~31.6%     |

**Key providers:**
- `Microsoft-Windows-Hyper-V-VMMS` — Core VMMS ETW provider (classic EventId-based events)
- `Microsoft.Windows.HyperV.Management` — WIL/TraceLogging-based management events (FallbackError, etc.)
- `Microsoft.Windows.HyperV.VmPhu` — VM Physical Hardware Unit events
- `Microsoft.Windows.HyperV.VmNetMgmt` — VM networking management events

**Companion tables in the same cluster/database:**
| Table Name | Description |
|---|---|
| **HyperVWorkerTable** | Events from worker process (`vmwp.exe`), except EventId 23100–23145 |
| **HyperVHypervisorTable** | Events from the hypervisor (hvix64.exe / hvax64.exe / hvaa64.exe) |
| **HyperVStorageStackTable** | Filtered storage virtualization provider events |
| **HyperVVPciTable** | Events for VPCI and device assignment |
| **HyperVVidTable** | Filtered events from virtualization driver interface (vid.sys/vid.dll) |
| **HyperVComputeTable** | Select events from Host Compute Service (`vmcompute.exe`) |
| **HyperVConfigTable** | Events from VM configuration/runtime file management |
| **HyperVAnalyticEvents** | Additional analytic events from Hyper-V |
| **HyperVTdprEvents** | TDPR-style timelines/graphs based on HyperV.Regions.xml |
| **UnderhillEventTable** | Events from guest VTL2 (Underhill) — in `wdgeventstore.kusto.windows.net` / `AzureHostOs` |

---

## 2. Schema

| Column               | Type     | Description |
|----------------------|----------|-------------|
| TIMESTAMP            | datetime | Ingestion timestamp |
| PreciseTimeStamp     | datetime | Event occurrence time (primary time filter) |
| Environment          | string   | Deployment environment (e.g., `PROD`) |
| Region               | string   | Azure region (e.g., `eastus`, `centralus`) |
| DataCenter           | string   | Physical datacenter code (e.g., `BL2`, `DSM41`) |
| Cluster              | string   | Azure cluster name (e.g., `BL2PrdApp02`) |
| NodeIdentity         | string   | Host node IP address |
| NodeId               | string   | Unique host node GUID |
| DeviceId             | string   | Device identifier (e.g., `s:<GUID>`) |
| Level                | long     | ETW severity: 1=Critical, 2=Error, 3=Warning, 4=Info, 5=Verbose |
| ProviderGuid         | string   | ETW provider GUID |
| ProviderName         | string   | ETW provider name |
| EventId              | long     | ETW event identifier (null for TraceLogging providers) |
| Pid                  | long     | Process ID of vmms.exe |
| Tid                  | long     | Thread ID |
| OpcodeName           | string   | ETW opcode name (often empty) |
| KeywordName          | string   | ETW keyword name (often empty) |
| TaskName             | string   | ETW task name — critical for categorizing events |
| ChannelName          | string   | ETW channel (Analytic, Operational, Admin, Networking, Storage) |
| EventMessage         | string   | Human-readable event message (classic ETW events) |
| ActivityId           | string   | Correlation GUID for tracking related operations |
| Task                 | long     | Numeric task identifier |
| Opcode               | long     | Numeric opcode |
| RelatedActivityId    | string   | Related activity GUID for parent-child correlation |
| Message              | string   | JSON-structured detailed payload (TraceLogging/rich events) |
| __AuthType__         | string   | Authentication type (e.g., `APPKI`) |
| __AuthIdentity__     | string   | Authentication identity chain |
| SourceNamespace      | string   | Source namespace (always `Fa`) |
| SourceMoniker        | string   | Source moniker (e.g., `FaDiagbl03`) |
| SourceVersion        | string   | Source version (e.g., `Ver249v0`) |
| AutopilotEnvironment | string   | Autopilot environment identifier |
| ObfuscatedData       | string   | Obfuscated data field (typically empty) |

---

## 3. Critical Column Guide — What to Query By

### Time Filtering
Always filter by `PreciseTimeStamp` first — this is the partitioning key that makes queries efficient.

### Identifying the Host
- **NodeId** — Unique GUID for the host node. Best for precise host lookups.
- **Cluster** — Azure cluster name. Best for scoping to a fleet segment.
- **Region** / **DataCenter** — Geographic scoping.

### Event Classification
- **Level** — Severity filter. Start with `Level <= 2` for errors, `Level == 3` for warnings.
- **EventId** — Specific event type identifier (null for TraceLogging providers).
- **TaskName** — Functional category of the event. Very useful for filtering by operation type.
- **ProviderName** — Which component emitted the event.
- **ChannelName** — ETW channel grouping (Analytic, Operational, Admin, Networking, Storage).

### Correlation
- **ActivityId** — Groups related events within a single operation.
- **RelatedActivityId** — Links parent-child operation chains.
- **Pid** / **Tid** — Process and thread IDs for threading analysis.

### Content
- **EventMessage** — Human-readable message for classic ETW events (EventId-based). May be empty for TraceLogging providers.
- **Message** — JSON payload for rich/TraceLogging events. Contains VmId, ContainerId, TaskId, error codes, etc. Parse with `parse_json(Message)`.
- **Always use fallback pattern:** `iif(isnotempty(EventMessage), EventMessage, Message)` or `coalesce(EventMessage, Message)`.

### Opcode Semantics
| Opcode | Meaning |
|--------|---------|
| **0** | General event (no start/end semantic) |
| **1** | **Start** of an activity/task |
| **2** | **End** of an activity/task |

If you see Opcode 1 (Start) without a matching Opcode 2 (End), the task may be stuck. Time between Start and End = task duration.

---

## 4. Key Event IDs Reference

### Highest Volume Events (last 24h)
| EventId | Count (~)      | Description |
|---------|----------------|-------------|
| 1801    | 67.6 billion   | General VMMS trace/diagnostic messages (WMI operations, provider queries) |
| null    | 65.4 billion   | TraceLogging events (no EventId — use TaskName/ProviderName instead) |
| 1101    | 6.9 billion    | Firmware boot order issues ("boot entry missing from boot order") |
| 1102    | 4.0 billion    | Related firmware/configuration events |
| 0       | 1.7 billion    | Generic/unclassified events |
| 12170   | 304 million    | VM management operations |
| 36000   | 148 million    | VM management operations |
| 18304   | 76 million     | VM management operations |
| 13003   | 74 million     | VM management operations |
| 12160   | 74 million     | VM management operations |
| 13002   | 74 million     | VM management operations |
| 21371   | 65 million     | VM state change events |
| 21370   | 65 million     | VM state change events |
| 21350   | 65 million     | VM state change events |
| 21351   | 65 million     | VM state change events |
| 21352   | 65 million     | VM state change events |
| 21353   | 65 million     | VM state change events |
| 21362   | 65 million     | VM state change events |
| 21395   | 65 million     | VM state change events |
| 21397   | 65 million     | VM state change events |

### Key TaskName Values
| TaskName | Count (~) | Description |
|----------|-----------|-------------|
| (empty) | 79.2 billion | Classic ETW events without TaskName |
| VmmsIndicateVmStateChange | 15.1 billion | VM state transitions. Message JSON contains `VmId`, `State`, `Reason`, `TaskId`, `TaskTypeName` |
| TaskCreated | 8.3 billion | Internal task creation tracking |
| TaskFinished | 8.0 billion | Internal task completion tracking |
| TaskCompleted | 7.5 billion | Signals end of a VMMS task. JSON contains `TaskID`, `ParentTaskID`, `TaskSubmitTime`, `TaskStartTime`, `TaskElapsedTime`, `TaskResultCode`, `TaskType`, `TaskTypeName`, `AssociatedObjectId` |
| WmiMethodExec | 5.8 billion | WMI method execution events |
| VmmsAutomaticManagementVtlReloadDispatch | 4.1 billion | VMMS task for **implicit/automatic** Underhill servicing. Tracks dispatch and overall servicing result |
| FallbackError | 1.7 billion | WIL error telemetry with HRESULT codes and source file/line info |
| WriteAttachmentInfo | 1.1 billion | Storage attachment information |
| PVM Realize | 1.0 billion | Planned VM realization operations |
| GetIovOffloadWeight | 925 million | SR-IOV network offload weight queries |
| AddVirtualSystemResource | 852 million | Resource addition to VMs |
| AddOneResourceSettings | 834 million | Individual resource settings configuration |
| AddResourceSettings | 819 million | Batch resource settings configuration |
| DisabledByHostwidePolicy | 554 million | Features disabled by host policy |

### TSG-Documented TaskNames (from investigation guides)
| TaskName | Context | Description |
|----------|---------|-------------|
| `VmmsIndicateVmStateChange` | Stop Container, General | VM state transition. Message JSON has `VmId`, `State`, `Reason`, `TaskId`, `TaskTypeName` |
| `TaskCompleted` | Stop Container, General | End of VMMS task. JSON has `TaskID`, `TaskResultCode` (0=success), `TaskElapsedTime`, `TaskTypeName` |
| `VmControl` | Underhill Servicing | General VM control. Filter `Message contains "ReloadManagementVtl"` for servicing calls |
| `WmiVirtualSystemSetting` | Underhill Servicing | WMI queries to VM settings. Used to look up `ManagementVtlUpdatePolicy` on the VSSD |
| `VmmsAutomaticManagementVtlReloadDispatch` | Underhill Servicing (implicit) | Automatic Underhill servicing. `wilActivity_hresult = 0` in stop event = overall success |
| `ReloadManagementVtlVmmsTaskDispatch` | Underhill Servicing (explicit) | Explicit Underhill servicing. HResult 4096 in stop event = successful dispatch to worker (does NOT indicate overall success) |
| `VmWorkerStateChange` | Migration (guest reset) | Logged at Level 5 during state changes |
| `VdevOperation` | Migration | Virtual device operations (save/restore). Opcode 1=Start, Opcode 2=Stop. Missing Opcode 2 = stuck device |

### Error-Specific TaskNames (Level <= 2)
| TaskName | Count (~) | Description |
|----------|-----------|-------------|
| (empty) | 4.0 billion | Classic ETW errors |
| FallbackError | 1.7 billion | WIL fallback errors with HRESULTs and source locations |
| TaskCompleted | 1.9 million | Tasks completed with error status |
| ActivityError | 967K | Activity-level errors |
| TaskCancelled | 397K | Cancelled tasks |

---

## 5. Common Message Patterns

### EventMessage (classic ETW — EventId-based)
- `"Attempt to complete a WMI operation that has already been completed - ignored!"` — Very common warning (EventId 1801)
- `"Provider could not handle query. Query = select ... from Msvm_ComputerSystem where ..."` — WMI query handling issues (EventId 1801)
- `"VmFirmwareBootOrderAccessor::GetOrderedEntries: Firmware boot entry missing from boot order, likely removed by guest."` — Guest modified boot order (EventId 1101)
- `"Vml::anonymous-namespace::VmpWbemRpnEncodedQuery::GetSelectQueries got unhandled WHERE clause operator"` — WMI query parsing issues (EventId 1801)

### Message (JSON — TraceLogging events)
The `Message` column contains structured JSON. Key patterns:

**Trace events (EventId 1801):**
```json
{
  "TraceData": "...",
  "VmName": "...",
  "VmId": "...",
  "StackFrameCount": N,
  "StackFrame_0": ...,
  "ModuleCount": N,
  "Module_0_Name": "D:\\Windows\\system32\\vmms.exe",
  ...
}
```

**FallbackError events (WIL errors):**
```json
{
  "PartA_PrivTags": 16777216,
  "wilResult_hresult": 2147749889,
  "wilResult_fileName": "onecore\\vm\\vmms\\wpmgr\\vmmsworkerprocess.cpp",
  "wilResult_lineNumber": 4395,
  "wilResult_module": "vmms.exe",
  "wilResult_failureType": 0,
  "wilResult_failureId": 19356,
  "wilResult_failureCount": 5026
}
```

**Parsing JSON Message in queries:**
```kql
| extend parsed = parse_json(Message)
| extend hresult = tolong(parsed.wilResult_hresult)
| extend sourceFile = tostring(parsed.wilResult_fileName)
```

---

## 6. Sample Queries

### 6.1 Basic VMMS Query Pattern
```kql
let fn_faultTime = datetime(YYYY-MM-DDTHH:MM:SSZ);
let fn_delta = 3h;
let fn_startTime = fn_faultTime - fn_delta;
let fn_endTime = fn_faultTime + fn_delta;
let fn_nodeId = "<node-guid>";
let fn_containerId = "<container-guid>";
let fn_vmId = "<vm-guid>";
cluster('azcore.centralus.kusto.windows.net').database("Fa").HyperVVmmsTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where Message has_any (fn_containerId, fn_vmId)
| project PreciseTimeStamp, Pid, Tid, ProviderName, EventId, TaskName, Level, Opcode,
    EventMessage = iif(isnotempty(EventMessage), EventMessage, Message)
| order by PreciseTimeStamp asc
```

### 6.2 Stop Container — Check if Shutdown Was Successful
```kql
let fn_nodeId = "<node-guid>";
let fn_containerId = "<container-guid>";
let fn_faultTime = datetime(YYYY-MM-DDTHH:MM:SSZ);
let fn_startTime = fn_faultTime - 1.3h;
let fn_endTime = fn_faultTime + 1h;
let fn_vmId = "<vm-guid>";
cluster('azcore.centralus').database('Fa').HyperVVmmsTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where Message has_any (fn_containerId, fn_vmId)
| where TaskName == "VmmsIndicateVmStateChange" or TaskName == "TaskCompleted"
| project PreciseTimeStamp, Pid, Tid, ProviderName, EventId, TaskName, Level, Opcode,
    EventMessage = iif(isnotempty(EventMessage), EventMessage, Message)
| order by PreciseTimeStamp asc
```

**What to look for:**
- Beginning: `VmmsIndicateVmStateChange` with `"Reason": "Task started"` and `"TaskTypeName": "Turning Off"`
- End: `TaskCompleted` matching the `TaskId` — check `TaskResultCode` (0=success) and `TaskElapsedTime`
- If VMMS shows success but HostAgent thinks container isn't stopped → route to **OneFleet Node/AzureHost-VMService**
- If VMMS shows error → cross-reference with **HyperVWorkerTable**

### 6.3 Live Migration — Common LM Query (VMMS + Worker + VID)
```kql
let fn_faultTime = datetime(YYYY-MM-DDTHH:MM:SSZ);
let fn_startTime = fn_faultTime - 1s;
let fn_endTime = fn_faultTime + 1s;
let fn_nodeIdSrc = "<source-node-guid>";
let fn_containerIdSrc = "<source-container-guid>";
let fn_nodeIdDest = "<dest-node-guid>";
let fn_containerIdDest = "<dest-container-guid>";
let fn_vmId = "<vm-guid>";
union
    (cluster('azcore.centralus').database('Fa').HyperVVmmsTable
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
        and NodeId in (fn_nodeIdSrc, fn_nodeIdDest)
        and Message has_any(fn_containerIdSrc, fn_containerIdDest, fn_vmId)),
    (cluster('azcore.centralus').database('Fa').HyperVWorkerTable
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
        and NodeId in (fn_nodeIdSrc, fn_nodeIdDest)),
    (cluster('azcore.centralus').database('Fa').HyperVVidTable
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
        and NodeId in (fn_nodeIdSrc, fn_nodeIdDest))
| extend LMNode = iif(NodeId == fn_nodeIdSrc, "Source", "Destination")
| project PreciseTimeStamp, Pid, Tid, LMNode, ProviderName, EventId, TaskName, Level, Opcode,
    EventMessage = iif(isnotempty(EventMessage), EventMessage, Message)
| order by PreciseTimeStamp asc
```

> **Note:** In LM, not all logs have VM ID or Container ID. Source and destination container IDs are different. Use `has_any` with both.

### 6.4 Live Migration — Detailed Query (All Hyper-V Tables)
```kql
let fn_faultTime = datetime(YYYY-MM-DDTHH:MM:SSZ);
let fn_startTime = fn_faultTime - 2h;
let fn_endTime = fn_faultTime + 2h;
let fn_nodeIdSrc = "<source-node-guid>";
let fn_nodeIdDest = "<dest-node-guid>";
union
    (cluster('azcore.centralus').database('Fa').HyperVVmmsTable
    | project PreciseTimeStamp, NodeId, ProviderName, ChannelName, TaskName, Level, EventId,
        Pid, Tid, EventMessage, Message, RelatedActivityId, ActivityId),
    (cluster('azcore.centralus').database('Fa').HyperVWorkerTable
    | project PreciseTimeStamp, NodeId, ProviderName, ChannelName, TaskName, Level, EventId,
        Pid, Tid, EventMessage, Message, RelatedActivityId, ActivityId),
    (cluster('azcore.centralus').database('Fa').HyperVStorageStackTable
    | project PreciseTimeStamp, NodeId, ProviderName, ChannelName, TaskName, Level, EventId,
        Pid, Tid, EventMessage, Message, RelatedActivityId, ActivityId),
    (cluster('azcore.centralus').database('Fa').HyperVVidTable
    | project PreciseTimeStamp, NodeId, ProviderName, ChannelName, TaskName, Level, EventId,
        Pid, Tid, EventMessage, Message, RelatedActivityId, ActivityId),
    (cluster('azcore.centralus').database('Fa').HyperVHypervisorTable
    | project PreciseTimeStamp, NodeId, ProviderName, ChannelName, TaskName, Level, EventId,
        Pid, Tid, EventMessage, Message, RelatedActivityId, ActivityId),
    (cluster('azcore.centralus').database('Fa').HyperVAnalyticEvents
    | project PreciseTimeStamp, NodeId, ProviderName, ChannelName, TaskName, Level, EventId,
        Pid, Tid, EventMessage, Message, RelatedActivityId, ActivityId)
// Add time/node/container filters per your investigation
| order by PreciseTimeStamp asc
```

### 6.5 Guest Reset Detection in VMMS
```kql
let fn_faultTime = datetime(YYYY-MM-DDTHH:MM:SSZ);
let fn_startTime = fn_faultTime - 10m;
let fn_endTime = fn_faultTime + 10m;
let fn_nodeId = "<node-guid>";
cluster('azcore.centralus').database('Fa').HyperVVmmsTable
| where NodeId == fn_nodeId
    and PreciseTimeStamp between (fn_startTime .. fn_endTime)
    and Message contains "reset"
| project PreciseTimeStamp, Tid, Message
| order by PreciseTimeStamp asc
```
**Keywords:** `VmStateReasonMigrationSourceGuestReset`, `VmStateReasonGuestReset`, `VmWorkerStateChange`

### 6.6 VdevOperation Analysis (vmbus deadlock detection)
```kql
// After running the Common LM Query (6.3), filter for stuck devices:
| where LMNode == "Source" and TaskName == "VdevOperation"
```
If a vdev has Opcode 1 (Start) but never Opcode 2 (Stop), it's stuck. Example: vmbus not completing indicates VPCI deadlock.

### 6.7 Underhill Servicing — Check ManagementVtlUpdatePolicy
```kql
let fn_nodeId = "<node-guid>";
let fn_containerId = "<container-guid>";
cluster('azcore.centralus').database('Fa').HyperVVmmsTable
| where NodeId == fn_nodeId
| where TaskName contains "WmiVirtualSystemSetting"
| where Message contains "ManagementVtlUpdatePolicy"
| where Message contains fn_containerId
```

**ManagementVtlUpdatePolicy values:**
| Value | Meaning |
|---|---|
| `0` or `Default` | No restriction on servicing |
| `1` or `OfflineOnly` | VM **cannot** be serviced (prevents live Underhill reload) |

### 6.8 Underhill Servicing — Check if Servicing Was Attempted
```kql
let fn_impactTime = datetime(YYYY-MM-DDTHH:MM:SSZ);
let fn_startTime = fn_impactTime - 10m;
let fn_endTime = fn_impactTime + 10m;
let fn_nodeId = "<node-guid>";
let fn_vmId = "<vm-guid>";
cluster('azcore.centralus').database('Fa').HyperVVmmsTable
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId and Message has fn_vmId
| where TaskName == "VmmsAutomaticManagementVtlReloadDispatch"
    or TaskName == "ReloadManagementVtlVmmsTaskDispatch"
| project TIMESTAMP, TaskName, Opcode, Message, ActivityId, RelatedActivityId
```

**Interpreting Stop Events:**
| Task | Opcode 2 Meaning |
|---|---|
| `ReloadManagementVtlVmmsTaskDispatch` | HResult 4096 = successful **dispatch** to worker only. Failure = not dispatched. |
| `VmmsAutomaticManagementVtlReloadDispatch` | `wilActivity_hresult = 0` = overall servicing **succeeded** |

**Common error codes:**
- `E_INVALID_STATE` (`0x8007139f`) — VM was in an invalid state
- `VM_E_VTL2_NOT_AVAILABLE` (`0xc0370702`) — VM was not an Underhill VM

**Timeout:** PilotFish has a **60-second timeout** per Underhill servicing operation. Exceeding leads to cancellation and potential VM reset.

### 6.9 Underhill Servicing — Verify Call Reached Worker
```kql
let fn_startTime = datetime(YYYY-MM-DDTHH:MM:SSZ);
let fn_endTime = fn_startTime + 2m;
let fn_vmId = "<vm-guid>";
cluster('azcore.centralus').database('Fa').HyperVVmmsTable
| union cluster('azcore.centralus').database('Fa').HyperVWorkerTable
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where Message has fn_vmId
| where TaskName == "VmControl" and Message contains "ReloadManagementVtl"
| project TIMESTAMP, Message, TaskName
```

### 6.10 Combined All Hyper-V Tables Query (Underhill debugging)
```kql
let fn_nodeId = '<node-guid>';
let fn_containerId = '<container-guid>';
let fn_startTime = datetime(YYYY-MM-DDTHH:MM:SSZ) - 5m;
let fn_endTime = datetime(YYYY-MM-DDTHH:MM:SSZ) + 1m;
let fn_filter = dynamic(['vmid', 'vmname', 'virtualmachineid', 'virtualmachinename',
    'fields', 'level', 'timestamp', 'op_code', 'related_activity_id', 'activity_id']);
let uh = cluster('wdgeventstore.kusto.windows.net').database('AzureHostOs').UnderhillEventTable
    | where NodeId == fn_nodeId and VmName == fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | extend MessageParsed = parse_json(tolower(tostring(Message)))
    | extend InnerMessageParsed = parse_json(tolower(tostring(MessageParsed.message)))
    | extend Fields = bag_merge(MessageParsed, InnerMessageParsed)
    | extend Fields = bag_remove_keys(Fields, fn_filter)
    | extend Fields = bag_remove_keys(Fields, dynamic(['message']))
    | extend Fields = bag_merge(Fields, InnerMessageParsed.fields, MessageParsed.fields)
    | extend Message = tostring(Fields)
    | extend Table = "uh";
let vmms = cluster('azcore.centralus').database('Fa').HyperVVmmsTable
    | where NodeId == fn_nodeId and Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Message !contains "WHERE clause operator"
        and Message !contains "Provider could not handle query"
    | where Level <= 4
    | extend Table = "vmms";
let vmwp = cluster('azcore.centralus').database('Fa').HyperVWorkerTable
    | where NodeId == fn_nodeId and Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Level <= 4
    | extend Table = "vmwp";
let vmhv = cluster('azcore.centralus').database('Fa').HyperVHypervisorTable
    | where NodeId == fn_nodeId and Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Level <= 4
    | extend Table = "vmhv";
let vpci = cluster('azcore.centralus').database('Fa').HyperVVPciTable
    | where NodeId == fn_nodeId and Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Level <= 4
    | extend Table = "vpci";
union uh, vmms, vmwp, vmhv, vpci
| project PreciseTimeStamp, Table, Level, TaskName, Opcode,
    EventMessage = coalesce(EventMessage, Message), ActivityId, RelatedActivityId
```

### Analyze FallbackError HRESULT distribution
```kql
HyperVVmmsTable
| where PreciseTimeStamp > ago(1h)
| where TaskName == "FallbackError"
| extend parsed = parse_json(Message)
| extend hresult = tolong(parsed.wilResult_hresult)
| extend sourceFile = tostring(parsed.wilResult_fileName)
| summarize count() by hresult, sourceFile
| order by count_ desc
```

### Check errors by cluster
```kql
HyperVVmmsTable
| where PreciseTimeStamp > ago(1h)
| where Level <= 2
| summarize ErrorCount = count() by Cluster
| order by ErrorCount desc
| take 20
```

---

## 7. Cross-Table Query Patterns

### Cross-References to Other Tables

| Related Table | Relationship to HyperVVmmsTable |
|---|---|
| **HyperVWorkerTable** | Worker process (`vmwp.exe`) logs. VMMS dispatches tasks to worker; cross-reference by `Pid`, `Message` containing VmId/ContainerId, or `TaskName == "VmControl"`. Worker has detailed per-stage logging for servicing (EventId 5124=failed, 5126=success). |
| **HyperVVidTable** | VID driver events. Often unioned with VMMS + Worker for migration investigations. |
| **HyperVStorageStackTable** | Storage virtualization events. Used in detailed migration queries alongside VMMS. |
| **HyperVHypervisorTable** | Hypervisor events. Used in combined Underhill and detailed migration queries. |
| **HyperVVPciTable** | VPCI/device assignment events. Used in Underhill combined query. Important for diagnosing vmbus deadlocks. |
| **HyperVAnalyticEvents** | Additional analytic events. Included in detailed LM query. |
| **HyperVConfigTable** | VM configuration file events (vsconfig.dll, vmdatastore.dll). |
| **HyperVVmConfigSnapshot** | VM configuration snapshot. Used to check `IsUnderhill` flag with `SummaryType == "Configuration"`. |
| **UnderhillEventTable** | Guest VTL2 events. Different cluster: `wdgeventstore.kusto.windows.net` / `AzureHostOs`. Uses `VmName` (== ContainerId) for filtering. |
| **HyperVEvents** (SharedWorkspace function) | Aggregated Hyper-V events. Used for shutdown type detection (EventId 18504=host, 18508=guest). |
| **LiveMigrationSessionCompleteLog** | LM session completion data. In `Fc` database. |
| **AirLiveMigrationEvents** | Detailed LM metrics (brownout, blackout, port programming delay). In `Air` database. |
| **MycroftContainerSnapshot** | Container VM type, Trusted VM status, subscription IDs. In `AzureCP` database. |
| **MycroftContainerHealthSnapshot** | What control layers believe about container state. In `AzureCP` database. |
| **IfxOperationV2v1EtwTable** | HostAgent operation errors. In `Fa` database. |
| **OsFileVersionTable** | File version info. Used to check vmfirmwareigvm.dll version for Underhill servicing. |

### Worker Process Follow-Up for Stop Container
```kql
let fn_nodeId = "<node-guid>";
let fn_containerId = "<container-guid>";
let fn_faultTime = datetime(YYYY-MM-DDTHH:MM:SSZ);
let fn_startTime = fn_faultTime - 1.3h;
let fn_endTime = fn_faultTime + 2h;
let pid = toscalar(
    cluster('azcore.centralus').database('Fa').HyperVWorkerTable
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime) and NodeId == fn_nodeId
    | where Message has fn_containerId
    | project Pid
);
cluster('azcore.centralus').database('Fa').HyperVWorkerTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime) and NodeId == fn_nodeId
| where Pid == pid
| project TIMESTAMP, Level, TaskName, Opcode, Message, EventMessage
| order by TIMESTAMP asc
```

### Guest Reset — Worker Correlation (Event 18514)
```kql
let fn_faultTime = datetime(YYYY-MM-DDTHH:MM:SSZ);
let fn_startTime = fn_faultTime - 10m;
let fn_endTime = fn_faultTime + 10m;
let fn_nodeId = "<node-guid>";
cluster('azcore.centralus').database('Fa').HyperVWorkerTable
| where NodeId == fn_nodeId
    and PreciseTimeStamp between (fn_startTime .. fn_endTime)
    and EventId == "18514"
| project PreciseTimeStamp, EventId, EventMessage
| order by PreciseTimeStamp asc
```
> **Note:** Event 18514 covers both LM aborted due to guest reset AND regular guest reset (no LM). LM abort is a side effect.

---

## 8. Investigation Playbook

### 8.1 Stop Container Failure
1. Run the stop-container query (Section 6.2) filtering for `VmmsIndicateVmStateChange` and `TaskCompleted`.
2. Look for `"Reason": "Task started"` with `"TaskTypeName": "Turning Off"` and match its `TaskId` to `TaskCompleted`.
3. Check `TaskResultCode` — 0 = success, non-zero = failure.
4. Check `TaskElapsedTime` for reasonableness.
5. If VMMS shows success but HostAgent disagrees → route to **OneFleet Node/AzureHost-VMService-Sev-3-4** or **Sev-1-2**.
6. If VMMS shows error → cross-reference **HyperVWorkerTable** using the Pid from the worker process.
7. Check if host-initiated (EventId 18504) vs. guest-initiated (EventId 18508) shutdown using the `HyperVEvents` function.

### 8.2 Live Migration Failure
1. Start with the Common LM Query (Section 6.3) — unions VMMS + Worker + VID across source and destination.
2. Tag rows as "Source" or "Destination" using `LMNode = iif(NodeId == fn_nodeIdSrc, "Source", "Destination")`.
3. Look for errors. Key keywords: `reset`, `failed`, `error`, `abort`.
4. For guest reset during LM: use Section 6.5. Event 18514 in worker = guest reset.
5. For VdevOperation issues: use Section 6.6, check for devices with Opcode 1 but no Opcode 2 (stuck).
6. For deeper dives: use Detailed LM Query (Section 6.4) adding StorageStack, Hypervisor, and Analytic tables.

**Key notes:**
- Source and destination container IDs are **different** in LM
- Not all LM logs contain VM ID or Container ID
- Large VMs (TB range) can have brownout periods up to ~9 hours

### 8.3 Underhill Servicing
**Two types:** Explicit (`ReloadManagementVtlVmmsTaskDispatch`) and Implicit/Automatic (`VmmsAutomaticManagementVtlReloadDispatch`).

1. **Check policy first** (Section 6.7) — is `ManagementVtlUpdatePolicy` blocking servicing?
2. **Check if servicing was attempted** (Section 6.8) during the impact window.
3. **Verify call reached worker** (Section 6.9) — look for `VmControl` + `ReloadManagementVtl`.
4. **Check worker stages** — each stage logged with Opcode 1 (start) and Opcode 2 (end). Missing Opcode 2 = stuck.
5. **Check worker result events:** EventId 5124 = **failed**, EventId 5126 = **successful**.
6. **Dive into Underhill logs** using correlation ID from event 5124/5126 to trace in UnderhillEventTable.

**Internal servicing mechanism (VMMS path):**
1. Call enters VMMS → checks if VM is Underhill, if loaded version < vmfirmwareigvm.dll version, if VM is Running.
2. If checks pass → dispatched to worker. Otherwise error returned.
3. Worker checks Underhill status and servicing policy, loads IGVM file, compares versions.
4. Worker instructs management VTL to save state.
5. **Point of no return** — any failure after state save requires VM reset.
6. Management VTL is reloaded with new firmware.

**Timeout:** PilotFish 60-second timeout → exceeding causes cancellation → potential VM reset.

### 8.4 General Error Investigation
```kql
// Step 1: Scope the problem
HyperVVmmsTable
| where PreciseTimeStamp between (datetime(YYYY-MM-DD HH:MM) .. datetime(YYYY-MM-DD HH:MM))
| where NodeId == "<node-guid>"
| where Level <= 2
| summarize count() by bin(PreciseTimeStamp, 5m), TaskName
| render timechart
```

```kql
// Step 2: Identify top error categories
HyperVVmmsTable
| where PreciseTimeStamp between (datetime(YYYY-MM-DD HH:MM) .. datetime(YYYY-MM-DD HH:MM))
| where NodeId == "<node-guid>"
| where Level <= 2
| summarize count() by TaskName, EventId
| order by count_ desc
```

```kql
// Step 3: Drill into FallbackError details
HyperVVmmsTable
| where PreciseTimeStamp between (datetime(YYYY-MM-DD HH:MM) .. datetime(YYYY-MM-DD HH:MM))
| where NodeId == "<node-guid>"
| where TaskName == "FallbackError"
| extend parsed = parse_json(Message)
| extend hresult = tolong(parsed.wilResult_hresult)
| extend sourceFile = tostring(parsed.wilResult_fileName)
| extend lineNumber = toint(parsed.wilResult_lineNumber)
| summarize count() by hresult, sourceFile, lineNumber
| order by count_ desc
```

```kql
// Step 4: Trace a specific operation via ActivityId
let activityId = "<activity-guid>";
HyperVVmmsTable
| where PreciseTimeStamp > ago(1d)
| where ActivityId == activityId or RelatedActivityId == activityId
| project PreciseTimeStamp, Level, TaskName, EventId, EventMessage, Message
| order by PreciseTimeStamp asc
```

---

## 9. When to Use This Table

**Use HyperVVmmsTable when investigating:**
- VM lifecycle failures (create, start, stop, delete, migrate not completing)
- WMI/CIM management interface issues
- VMMS service crashes or error spikes
- VM configuration and resource allocation problems (storage, network, firmware)
- VTL (Virtual Trust Level) management and reload issues (Underhill servicing)
- SR-IOV / network offload configuration
- Host-wide policy enforcement on VMs
- Worker process management issues (`vmmsworkerprocess.cpp` errors)
- Boot order and firmware-related issues

**Do NOT use this table for:**
- Guest OS-level events → Use `UnderhillEventTable` (`wdgeventstore.kusto.windows.net` / `AzureHostOs`)
- Hypervisor-level issues (scheduling, memory partitioning) → Use `HyperVHypervisorTable`
- VM worker process runtime events → Use `HyperVWorkerTable`
- Storage stack I/O path issues → Use `HyperVStorageStackTable`
- VPci/device assignment issues → Use `HyperVVPciTable`

**ETW Channel Guidance:**
| Channel | Use When |
|---------|----------|
| `Microsoft-Windows-Hyper-V-VMMS-Analytic` | High-volume diagnostic traces, WMI operation details |
| `Microsoft-Windows-Hyper-V-VMMS-Operational` | Standard operational events |
| `Microsoft-Windows-Hyper-V-VMMS-Admin` | Administrative events (visible in Event Viewer) |
| `Microsoft-Windows-Hyper-V-VMMS-Networking` | Network-specific VMMS operations |
| `Microsoft-Windows-Hyper-V-VMMS-Storage` | Storage-specific VMMS operations |
| `Microsoft-Windows-Hyper-V-High-Availability-Admin` | HA/clustering admin events |
| `Microsoft-Windows-Hyper-V-High-Availability-Analytic` | HA/clustering diagnostic traces |

---

## 10. Important IDs and How They Relate

| ID | Description | Where Used |
|---|---|---|
| **NodeId** | Host node GUID | Primary filter on all queries |
| **ContainerId** | Container GUID (== `VmName` in Underhill) | Filter via `Message has fn_containerId` |
| **VmId** | Virtual Machine unique ID within Hyper-V | Filter via `Message has fn_vmId`. Different from ContainerId! |
| **VmUniqueId** | Azure-level VM resource ID (survives migrations) | Not directly in HyperVVmmsTable; use AzureCM tables |
| **TaskId** | VMMS task correlation ID | In `Message` JSON of `VmmsIndicateVmStateChange` and `TaskCompleted` |
| **ActivityId / RelatedActivityId** | ETW activity correlation | Tracing cross-process operations |
| **Correlation ID** | Underhill servicing correlation | In worker EventId 5124/5126; used to trace into UnderhillEventTable |

> **Caution:** Pay attention to the distinction between **VM ID** and **VM name == container ID**. See the [Underhill Kusto FAQ](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-kusto-queries-faq).

---

## 11. Tips and Gotchas

1. **Message field is JSON** — Use `parse_json(Message)` to extract structured fields. Use `has` or `has_any` for GUID matching (faster than `contains`).

2. **EventMessage vs Message** — Some events populate `EventMessage`, others only `Message`. Always use: `iif(isnotempty(EventMessage), EventMessage, Message)` or `coalesce(EventMessage, Message)`.

3. **Noisy events pre-filtered** — Some noisy events are already filtered at ingestion. For combined queries, also exclude `"WHERE clause operator"` and `"Provider could not handle query"` spam.

4. **Level filtering** — For combined queries, `Level <= 4` excludes verbose. But for stop-container investigation, Level 5 events like `VmmsIndicateVmStateChange` are **critical** — don't filter them out!

5. **Opcode semantics** — Opcode 1 = Start, Opcode 2 = End. Start without End = stuck task.

6. **HResult interpretation for servicing:**
   - `ReloadManagementVtlVmmsTaskDispatch` stop event HResult 4096 = successful **dispatch only**
   - `VmmsAutomaticManagementVtlReloadDispatch` stop event `wilActivity_hresult = 0` = overall success

7. **60-second PilotFish timeout** — Exceeding causes cancellation, potential VM reset. Check for missing Opcode 2 events.

8. **Point of no return in servicing** — After management VTL save state, any failure requires VM reset.

9. **Migration: Source vs Destination** — Always tag nodes: `LMNode = iif(NodeId == fn_nodeIdSrc, "Source", "Destination")`.

10. **ContainerId differs between source and destination** in LM. Use `has_any(fn_containerIdSrc, fn_containerIdDest, fn_vmId)`.

11. **Not all LM logs have VM ID** — May need to query without container filters and rely on time windows.

12. **Time windows vary by scenario:**
    - Stop container: ±1–1.3h from fault time
    - Migration: ±2h for container-filtered, ±25min for all logs (wider for large VMs)
    - Underhill servicing: ±10min from impact time
    - Guest reset: ±10min from fault time

13. **Determining Underhill VMs** — Check `HyperVVmConfigSnapshot` with `SummaryType == "Configuration"` and `parse_json(SummaryJson).Settings.hcl.IsUnderhill`.

14. **SharedWorkspace functions** — The Hyper-V SME team maintains shared Kusto functions in the azcore cluster SharedWorkspace database.

---

## 12. Maintainer

- **Contact:** hypsme
- **IcM Queue:** RDOS/Azure Host OS SME - Virtualization (Hyper-V)
- **Routing:** For guest resets, route to WSD CFE\HCCompute-Guest OS Health (Windows) or LSG/Triage (Linux). See [aka.ms/rdosroute](https://aka.ms/rdosroute).
