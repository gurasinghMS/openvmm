# HyperVVmmsTable Investigation Guide

> Distilled from RDOS Livesite EngHub documentation.
> Covers: HyperVVmmsTable (vmms.exe logging) in azcore.centralus / Fa
>
> **Sources:**
> - [Hyper-V Kusto Queries](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/hyperv-kusto-queries)
> - [Stop Container Failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/stop-container-failure)
> - [Migration Failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/migration/migration-failure)
> - [Underhill Servicing TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-servicing)
> - [Underhill Kusto Queries FAQ](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-kusto-queries-faq)

---

## 1. Table Overview

**HyperVVmmsTable** contains events and traces from Hyper-V's **Virtual Machine Management Service (vmms.exe)**. This table has filtered out a few noisy events.

| Property | Value |
|---|---|
| **Process** | `vmms.exe` — Hyper-V Virtual Machine Management Service |
| **Cluster** | `azcore.centralus.kusto.windows.net` |
| **Database** | `Fa` |
| **Full Path** | `cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVVmmsTable` |
| **Provider** | `Microsoft.Windows.HyperV.Management` |
| **Noise Filtering** | Some noisy events are pre-filtered out of this table |

The table sits alongside other Hyper-V tables in the same cluster/database:

| Table Name | Description |
|---|---|
| **HyperVTdprEvents** | TDPR-style timelines/graphs based on HyperV.Regions.xml |
| **HyperVWorkerTable** | Events from worker process (`vmwp.exe`), except EventId 23100–23145. Payload in JSON `Message` column |
| **HyperVVmmsTable** | Events from VMMS (`vmms.exe`). Some noisy events filtered out |
| **HyperVComputeTable** | Select events from Host Compute Service (`vmcompute.exe`). Narrow collection due to noisiness |
| **HyperVConfigTable** | Events from VM configuration/runtime file management (vsconfig.dll, vmdatastore.dll) |
| **HyperVHypervisorTable** | Events from the hypervisor (hvix64.exe / hvax64.exe / hvaa64.exe) |
| **HyperVVidTable** | Filtered events from virtualization driver interface (vid.sys/vid.dll) |
| **HyperVStorageStackTable** | Filtered storage virtualization provider events. Payload in JSON `Message` column |
| **HyperVVPciTable** | Events for VPCI and device assignment. Payload in JSON `Message` field |
| **UnderhillEventTable** | Events from guest VTL2 (Underhill) — in `wdgeventstore.kusto.windows.net` / `AzureHostOs` |
| **HyperVAnalyticEvents** | Additional analytic events from Hyper-V |

---

## 2. Key Fields

The following fields are commonly projected and filtered on in HyperVVmmsTable queries:

| Field | Description |
|---|---|
| `PreciseTimeStamp` | High-precision timestamp of the event |
| `TIMESTAMP` | Standard timestamp (also usable for time range filtering) |
| `NodeId` | The host node GUID |
| `Pid` | Process ID of vmms.exe |
| `Tid` | Thread ID |
| `ProviderName` | ETW provider name (e.g., `Microsoft.Windows.HyperV.Management`) |
| `ChannelName` | ETW channel |
| `EventId` | Numeric event identifier |
| `TaskName` | Name of the VMMS task (see Key TaskNames section below) |
| `Level` | Event level (5 = Verbose/Informational) |
| `Opcode` | **1 = Start, 2 = End** of an activity |
| `Message` | Raw event payload (often JSON). Contains VmId, ContainerId, TaskId, error codes, etc. |
| `EventMessage` | Structured event message (may be empty; fall back to `Message`) |
| `ActivityId` | Activity correlation GUID |
| `RelatedActivityId` | Related activity correlation GUID |

### Projecting EventMessage with Fallback

A common pattern is:
```kusto
| project PreciseTimeStamp, Pid, Tid, ProviderName, EventId, TaskName, Level, Opcode,
    EventMessage = iif(isnotempty(EventMessage), EventMessage, Message)
```

Or using `coalesce`:
```kusto
| project PreciseTimeStamp, Table, Level, TaskName, Opcode,
    EventMessage = coalesce(EventMessage, Message), ActivityId, RelatedActivityId
```

---

## 3. Key TaskNames

The following `TaskName` values are referenced across the TSG documentation when querying HyperVVmmsTable:

| TaskName | Context | Description |
|---|---|---|
| `VmmsIndicateVmStateChange` | Stop Container, General | Indicates a VM state transition. The `Message` JSON contains `VmId`, `State`, `Reason`, `TaskId`, `TaskTypeName` |
| `TaskCompleted` | Stop Container, General | Signals the end of a VMMS task. JSON contains `TaskID`, `ParentTaskID`, `TaskSubmitTime`, `TaskStartTime`, `TaskElapsedTime`, `TaskResultCode`, `TaskType`, `TaskTypeName`, `AssociatedObjectIdType`, `AssociatedObjectId` |
| `VmControl` | Underhill Servicing | General VM control operations. Filter `Message contains "ReloadManagementVtl"` to find Underhill servicing calls dispatched to worker |
| `WmiVirtualSystemSetting` | Underhill Servicing | WMI-based queries to VM settings. Used to look up `ManagementVtlUpdatePolicy` on the VSSD |
| `VmmsAutomaticManagementVtlReloadDispatch` | Underhill Servicing (implicit) | VMMS task created for **implicit/automatic** Underhill servicing. Tracks both dispatch and overall servicing result |
| `ReloadManagementVtlVmmsTaskDispatch` | Underhill Servicing (explicit) | VMMS task created for **explicit** Underhill servicing. HResult 4096 in stop event indicates successful dispatch to worker |
| `VmWorkerStateChange` | Migration (guest reset) | Logged at Level 5 during state changes |
| `VdevOperation` | Migration | Virtual device operations (save/restore). Opcode 1=Start, Opcode 2=Stop |

---

## 4. Opcode Meanings

| Opcode | Meaning |
|---|---|
| **1** | **Start** of an activity/task |
| **2** | **End** of an activity/task |
| **0** | General event (no start/end semantic, e.g., state change notifications) |

**Critical for Underhill servicing analysis:**
- `Opcode == 1` → Start of `VmmsAutomaticManagementVtlReloadDispatch` or `ReloadManagementVtlVmmsTaskDispatch`
- `Opcode == 2` → End of the activity. Check the `HResult`/`wilActivity_hresult` in the stop event.
  - For **explicit** servicing (`ReloadManagementVtlVmmsTaskDispatch`): HResult 4096 in the stop event indicates successful **dispatch** to worker (does NOT indicate overall success).
  - For **implicit** servicing (`VmmsAutomaticManagementVtlReloadDispatch`): `wilActivity_hresult = 0` in stop event indicates overall servicing success.

**Critical for stop-container analysis:**
- Time between the beginning event (state change "Task started") and the end event (`TaskCompleted`) represents the total shutdown duration.

**Critical for migration analysis:**
- `Opcode 1` = Start, `Opcode 2` = Stop for VdevOperations. If a vdev goes from opcode 1 but never reaches opcode 2, it indicates a stuck/failed operation.

---

## 5. Common Filtering Patterns

### Filter by Node and Container/VM ID
```kusto
HyperVVmmsTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where Message has_any (fn_containerId, fn_vmId)
```

### Filter by TaskName
```kusto
| where TaskName == "VmmsIndicateVmStateChange" or TaskName == "TaskCompleted"
```

### Filter for Underhill servicing tasks
```kusto
| where TaskName == "VmmsAutomaticManagementVtlReloadDispatch"
    or TaskName == "ReloadManagementVtlVmmsTaskDispatch"
```

### Filter by keyword in Message
```kusto
| where Message contains "reset"
| where Message contains "ManagementVtlUpdatePolicy"
| where Message contains "ReloadManagementVtl"
```

### Exclude noisy spam (for combined queries)
```kusto
| where Message !contains "WHERE clause operator"
    and Message !contains "Provider could not handle query"
```

### Filter by Level
```kusto
| where Level <= 4  // Exclude verbose/informational (Level 5) for less noise
```

---

## 6. All Kusto Query Examples

### 6.1 Basic HyperVVmmsTable Query (from Hyper-V Kusto Queries page)

> **Source:** [hyperv-kusto-queries](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/hyperv-kusto-queries)

The example on the overview page uses HyperVWorkerTable (not VmmsTable directly), but establishes the standard query pattern for all Hyper-V tables in the same cluster/database:

```kusto
// Cluster: azcore.centralus.kusto.windows.net / Database: Fa
let fn_faultTime = datetime(5/6/2023 12:12:49 PM);
let fn_delta = 3h;
let fn_startTime = fn_faultTime - fn_delta;
let fn_endTime = fn_faultTime + fn_delta;
let fn_nodeId = "9a9c671e-f9d5-4f4f-9eb4-8e968618124e";
let fn_containerId = "5954a243-632f-4604-9f7f-5be55b5e6685";
let fn_vmId = "d679e7e3-787a-46bb-badf-e21989a9568c";
cluster('azcore.centralus.kusto.windows.net').database("Fa").HyperVWorkerTable
| where PreciseTimeStamp between ((fn_startTime) .. (fn_endTime))
| where NodeId == fn_nodeId
| where Message has fn_containerId or Message has fn_vmId
| project PreciseTimeStamp, TaskName, EventId, Message, Opcode
```

The same pattern applies to HyperVVmmsTable — replace the table name.

---

### 6.2 Stop Container — Check if Shutdown Was Successful (VMMS Logs)

> **Source:** [stop-container-failure TSG, Step 8](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/stop-container-failure)

```kusto
// Cluster: azcore.centralus.kusto.windows.net / Database: Fa
let fn_nodeId = "77764c5d-a46e-2207-4857-e5cc64c7b154";
let fn_containerId = "05b62c9a-64f9-466e-89fc-c59586c7cf71";
let fn_faultTime = datetime(2023-03-31T02:42:00Z);
let fn_startTime = fn_faultTime - 1.3h;
let fn_endTime = fn_faultTime + 1h;
let fn_vmId = "c121eaab-529a-40b0-ab81-5f33af960559";
cluster('azcore.centralus').database('Fa').HyperVVmmsTable
| where (PreciseTimeStamp > fn_startTime) and (PreciseTimeStamp < fn_endTime) and NodeId == fn_nodeId
| where Message has_any (fn_containerId, fn_vmId)
| where TaskName == "VmmsIndicateVmStateChange" or TaskName == "TaskCompleted"
| project PreciseTimeStamp, Pid, Tid, ProviderName, EventId, TaskName, Level, Opcode,
    EventMessage = iif(isnotempty(EventMessage), EventMessage, Message)
| order by PreciseTimeStamp asc
```

**What to look for:** Beginning and end of VM shutdown tasks.

**Sample beginning logline (VmmsIndicateVmStateChange):**
```
2023-03-31T01:34:55.40642Z 328 26084 Microsoft.Windows.HyperV.Management
VmmsIndicateVmStateChange 5 0
{
  "VmId": "c121eaab-529a-40b0-ab81-5f33af960559",
  "State": "VmmsVmStateRunning",
  "Reason": "Task started",
  "TaskId": "7739c01d-e5f9-4ba2-b9e2-acd9fe366f37",
  "TaskTypeName": "Turning Off"
}
```

**Sample end logline (TaskCompleted):**
```
2023-03-31T01:35:01.3107434Z 328 18188 Microsoft.Windows.HyperV.Management
TaskCompleted 5 0
{
  "TaskID": "7739c01d-e5f9-4ba2-b9e2-acd9fe366f37",
  "ParentTaskID": "00000000-0000-0000-0000-000000000000",
  "TaskSubmitTime": "2023-03-31T01:34:55.4060562Z",
  "TaskStartTime": "2023-03-31T01:34:55.4060562Z",
  "TaskElapsedTime": 59042920,
  "TaskResultCode": 0,
  "TaskType": 5,
  "TaskTypeName": "Turning Off",
  "AssociatedObjectIdType": 1,
  "AssociatedObjectId": "c121eaab-529a-40b0-ab81-5f33af960559"
}
```

**Interpretation:**
- If **no error** in both events and the time between beginning and end is reasonable → container shutdown was **successful** from Hyper-V's perspective.
  - Route to **OneFleet Node/AzureHost-VMService-Sev-3-4** or **OneFleet Node/AzureHost-VMService-Sev-1-2** (depending on severity) for HostAgent to investigate why the agent believed the container wasn't stopped.
- If an **error is surfaced from VMMS**, cross-reference with **HyperVWorkerTable** to get more detail on origin.

**Follow-up: Worker process logs for stop-container errors:**
```kusto
let fn_nodeId = "77764c5d-a46e-2207-4857-e5cc64c7b154";
let fn_containerId = "05b62c9a-64f9-466e-89fc-c59586c7cf71";
let fn_faultTime = datetime(2023-03-31T02:42:00Z);
let fn_startTime = fn_faultTime - 1.3h;
let fn_endTime = fn_faultTime + 2h;
let pid = toscalar(
    cluster('azcore.centralus').database('Fa').HyperVWorkerTable
    | where (PreciseTimeStamp > fn_startTime) and (PreciseTimeStamp < fn_endTime) and NodeId == fn_nodeId
    | where Message has fn_containerId
    | project Pid
);
cluster('azcore.centralus').database('Fa').HyperVWorkerTable
| where (PreciseTimeStamp > fn_startTime) and (PreciseTimeStamp < fn_endTime) and NodeId == fn_nodeId
| where Pid == pid
| project TIMESTAMP, Level, TaskName, Opcode, Message, EventMessage
| sort by TIMESTAMP asc
```

---

### 6.3 Live Migration — Common LM Query (VMMS + Worker + VID)

> **Source:** [migration-failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/migration/migration-failure)

This is the **go-to starting query** for migration-related issues. It unions HyperVVmmsTable with HyperVWorkerTable and HyperVVidTable across source and destination nodes:

```kusto
// Cluster: azcore.centralus.kusto.windows.net / Database: Fa
let fn_faultTime = datetime(2023-07-20T13:31:06.7758456Z);
let fn_startTime = fn_faultTime - 1s;
let fn_endTime = fn_faultTime + 1s;
let fn_nodeIdSrc = "02786dd2-1dd9-89ee-57e5-7638b3736b51";
let fn_containerIdSrc = "80d8cd73-db9f-40d7-bb1d-8b0783004fd7";
let fn_nodeIdDest = "c8f5e105-d49d-d145-999b-3de509250e30";
let fn_containerIdDest = "ae81451e-3abc-4767-ad01-86cf156911dd";
let fn_vmId = "583b7d5e-6ff9-4744-b665-3e9c71ea4bd9";
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

**Key notes:**
- In a Live Migration (LM), **not all logs have a VM ID or a Container ID (VM name)**.
- Large VM sizes (TB range) can have long brownout periods (up to ~9 hours), affecting the time window.
- Storage issues or vdev issues (e.g., SCSI controller failed to restore) may appear during LM — all vdevs need to be saved and restored on the destination.

---

### 6.4 Live Migration — Detailed Query (VMMS + Worker + Storage + VID + Hypervisor + Analytic)

> **Source:** [migration-failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/migration/migration-failure)

More detailed than the common query; recommended for deeper dives:

```kusto
// Cluster: azcore.centralus.kusto.windows.net / Database: Fa
let fn_faultTime = datetime(8/24/2022 7:19:07 AM);
let fn_startTime = fn_faultTime - 2h;  // -2h for logs containing container/VM ID, or -25min for all logs
let fn_endTime = fn_faultTime + 2h;    // +2h for logs containing container/VM ID, or +25min for all logs
let fn_nodeIdSrc = "8a5d2dfd-b281-ea34-68c5-94b5aa11e80d";
let fn_containerIdSrc = "cf81a6a6-794a-4d4a-ba2f-0df072251f8a";
let fn_nodeIdDest = "58400f5c-303c-70ea-9327-20835e15c1a3";
let fn_containerIdDest = "589463e4-2b16-4aca-a21d-94003a336e99";
let fn_vmId = "3c429a60-6599-48a4-b412-98fd500e8dc7";
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
// Additional time/node/container filters should be added here per your investigation
| order by PreciseTimeStamp asc
```

**Note:** The detailed query exposes `ChannelName`, `RelatedActivityId`, and `ActivityId` which are critical for tracing cross-process activity correlation.

---

### 6.5 Live Migration — Guest Reset Detection in VMMS

> **Source:** [migration-failure TSG, Example 1 Step 2](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/migration/migration-failure)

Search for "reset" in the VMMS logs:

```kusto
// Cluster: azcore.centralus.kusto.windows.net / Database: Fa
let fn_faultTime = datetime(2023-07-01 05:10:25.1038514);
let fn_startTime = fn_faultTime - 10m;
let fn_endTime = fn_faultTime + 10m;
let fn_nodeId = "4b52c553-b810-ab43-c613-77ce5bc4d3a0";
cluster('azcore.centralus').database('Fa').HyperVVmmsTable
| where NodeId == fn_nodeId
    and PreciseTimeStamp >= fn_startTime
    and PreciseTimeStamp <= fn_endTime
    and Message contains "reset"
| sort by PreciseTimeStamp asc
| project PreciseTimeStamp, Tid, Message
| sort by PreciseTimeStamp asc
```

**Keywords to look for in VMMS logs:**
- `"VmStateReasonMigrationSourceGuestReset"`
- `"VmStateReasonGuestReset"`
- `"VmWorkerStateChange"` (Level 5)

**Corresponding worker query for Event 18514 (guest reset):**
```kusto
let fn_faultTime = datetime(2023-07-01 05:10:25.1038514);
let fn_startTime = fn_faultTime - 10m;
let fn_endTime = fn_faultTime + 10m;
let fn_nodeId = "4b52c553-b810-ab43-c613-77ce5bc4d3a0";
cluster('azcore.centralus').database('Fa').HyperVWorkerTable
| where NodeId == fn_nodeId
    and PreciseTimeStamp >= fn_startTime
    and PreciseTimeStamp <= fn_endTime
    and EventId == "18514"
| sort by PreciseTimeStamp asc
| project PreciseTimeStamp, EventId, EventMessage
| sort by PreciseTimeStamp asc
```

> **Note:** Event 18514 is used for both LM being aborted due to a guest reset event AND for a regular guest reset event while the VM is running (no LM in progress). LM being aborted is a side effect of the guest reset.

---

### 6.6 Live Migration — VdevOperation Analysis (vmbus deadlock detection)

> **Source:** [migration-failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/migration/migration-failure)

Filter for VdevOperations to identify stuck virtual devices (e.g., vmbus failing to power off due to VPCI deadlock):

```kusto
// Cluster: azcore.centralus.kusto.windows.net / Database: Fa
let fn_startTime = datetime(2024-01-14 02:13:16.77);
let fn_endTime = datetime(2024-01-14T04:04:15.170511Z);
let fn_nodeIdSrc = "fd746950-737c-a1aa-f7fa-26c7875883ce";
let fn_containerIdSrc = "e31ea074-1bd3-46ab-aee8-39e888b9168f";
let fn_nodeIdDest = "965e78c6-335c-a08d-4572-19bae2bba484";
let fn_containerIdDest = "a5cf69db-eb90-4014-9a8c-d59c92f9fd22";
let fn_lmSessionId = "7e8133dd-3cdb-4a5c-9c22-0d8c8a1d39cb";
let fn_vmId = "96394110-8c71-4c6d-9be0-0bf0f3a4b0f2";
let fn_migToSuspendTaskId = "58e43b85-63a2-4578-aa46-bc3c1c16a0a7";
union
    (cluster('azcore.centralus').database('Fa').HyperVVmmsTable
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
        and NodeId in (fn_nodeIdSrc, fn_nodeIdDest)
        and Message has_any(fn_containerIdSrc, fn_containerIdDest, fn_vmId, fn_migToSuspendTaskId)),
    (cluster('azcore.centralus').database('Fa').HyperVWorkerTable
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
        and NodeId in (fn_nodeIdSrc, fn_nodeIdDest))
| where not (EventMessage contains "The virtual machine is currently performing the following operation")
| where not (EventMessage contains "cannot start or queue new task [Modifying Resource] for action")
| where not (EventMessage contains "cannot start task [Modifying Resource]")
| extend LMNode = iif(NodeId == fn_nodeIdSrc, "Source", "Destination")
| project PreciseTimeStamp, Pid, Tid, LMNode, ProviderName, EventId, TaskName, Level, Opcode,
    EventMessage = iif(isnotempty(EventMessage), EventMessage, Message)
| order by PreciseTimeStamp asc
| where LMNode == "Source" and TaskName == "VdevOperation"
```

**Analysis:** If a vdev goes from Opcode 1 (Start) but never reaches Opcode 2 (Stop), it indicates a stuck device. For example, vmbus being the only vdev that doesn't complete its stop indicates a VPCI deadlock preventing vmbus from clearing.

---

### 6.7 Underhill Servicing — Check ManagementVtlUpdatePolicy (VSSD)

> **Source:** [underhill-servicing TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-servicing)

Look up whether a VM's Underhill servicing policy permits updates:

```kusto
// Cluster: azcore.centralus.kusto.windows.net / Database: Fa
let fn_nodeId = "2e63720c-77d6-28cc-421f-ef5e12a6ef7a";
let fn_containerId = "15077e9b-31ef-49e8-bca1-8f7382a56d4b";
// ContainerId or VmId should work here
cluster('azcore.centralus').database('Fa').HyperVVmmsTable
| where NodeId == fn_nodeId
| where TaskName contains "WmiVirtualSystemSetting"
| where Message contains "ManagementVtlUpdatePolicy"
| where Message contains fn_containerId
```

**ManagementVtlUpdatePolicy values:**
| Value | Meaning |
|---|---|
| `0` or `Default` | No restriction on servicing this VM |
| `1` or `OfflineOnly` | VM **cannot** be serviced (prevents live Underhill reload) |

- The per-VM setting is on the VM's VSSD (Virtual System Setting Data).
- A global registry key also exists: `HKLM\Software\Microsoft\Windows NT\CurrentVersion\Virtualization\ManagementVtlUpgradePolicy`
- If key doesn't exist or value is 0/Default → no restriction.
- If value > 0 or not Default → Underhill servicing is **not permitted** for any VM on the node and will fail.

---

### 6.8 Underhill Servicing — Check if Servicing Was Attempted During Impact Window

> **Source:** [underhill-servicing TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-servicing)

```kusto
// Cluster: azcore.centralus.kusto.windows.net / Database: Fa
let fn_impactTime = datetime("2024-05-07T16:35:19.5889444Z");
let fn_startTime = fn_impactTime - 10m;
let fn_endTime = fn_impactTime + 10m;
let fn_nodeId = "3294bd07-9f08-501d-d62c-c2bcea0cb027";
let fn_vmId = "cdc43451-f401-4a64-9795-ef5b2eb239a1"; // VM Name or ID; ID recommended
cluster('azcore.centralus').database('Fa').HyperVVmmsTable
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId and Message has fn_vmId
| where TaskName == "VmmsAutomaticManagementVtlReloadDispatch"
    or TaskName == "ReloadManagementVtlVmmsTaskDispatch"
| project TIMESTAMP, TaskName, Opcode, Message, ActivityId, RelatedActivityId
```

**Interpreting Opcode/HResult for servicing tasks:**

| Task | Opcode 1 | Opcode 2 (Stop Event) |
|---|---|---|
| `ReloadManagementVtlVmmsTaskDispatch` (explicit) | Start of dispatch | HResult 4096 = successful **dispatch** to worker. Does NOT mean servicing succeeded overall. If Stop event failed → servicing failed without being dispatched to worker. |
| `VmmsAutomaticManagementVtlReloadDispatch` (implicit) | Start of automatic servicing | `wilActivity_hresult = 0` = overall servicing operation **succeeded**. Also tracks VMWP result. |

**Timeout:** PilotFish package has a **60-second timeout** for each Underhill servicing operation. If servicing doesn't complete in this timeout, cancellation is sent to the virtualization stack, which may cause VM reset.

**Common error codes:**
- `E_INVALID_STATE` (`0x8007139f`) — VM was in an invalid state to start the task
- `VM_E_VTL2_NOT_AVAILABLE` (`0xc0370702`) — VM was not an Underhill VM

---

### 6.9 Underhill Servicing — VmControl + ReloadManagementVtl (VMMS + Worker Union)

> **Source:** [underhill-servicing TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-servicing)

Verify that the servicing call made it to the worker process:

```kusto
// Cluster: azcore.centralus.kusto.windows.net / Database: Fa
let fn_startTime = datetime(2024-05-14T07:15:06.4888086Z);
let fn_endTime = datetime(2024-05-14T07:17:06.4888086Z);
let fn_vmId = "158b311b-f347-4ffe-92f3-9b335e7c76d5";
cluster('azcore.centralus').database('Fa').HyperVVmmsTable
| union cluster('azcore.centralus').database('Fa').HyperVWorkerTable
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where Message has fn_vmId
| where TaskName == "VmControl" and Message contains "ReloadManagementVtl"
| project TIMESTAMP, Message, TaskName
```

**Interpretation:** Existence of `VmControl` task with `ReloadManagementVtl` in the Message field in VMWP logs indicates the Underhill servicing call successfully made it to the worker process. If this is absent, the task may have gotten stuck on the worker state dispatcher.

---

### 6.10 Underhill — Combined All HyperV Tables Query

> **Source:** [underhill-kusto-queries-faq](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-kusto-queries-faq)

A combined query that unions Underhill Event Table, HyperV VMMS Table, HyperV Worker Table, HyperV Hypervisor Table, and HyperV VPCI Table:

```kusto
let fn_nodeId = '94db1e7e-f598-4c19-ab60-a9f423a5e3ef';
let fn_containerId = '42390b9f-16fc-4761-8999-017175e7daf1';
let fn_startTime = datetime(2025-09-15 23:53:13) - 5m;
let fn_endTime = datetime(2025-09-15 23:53:13) + 1m;
let fn_filter = dynamic(['vmid', 'vmname', 'virtualmachineid', 'virtualmachinename',
    'fields', 'level', 'timestamp', 'op_code', 'related_activity_id', 'activity_id']);
let uh = cluster('wdgeventstore.kusto.windows.net').database('AzureHostOs').UnderhillEventTable
    | where NodeId == fn_nodeId
    | where VmName == fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    //| where Level <= 4
    | extend MessageParsed = parse_json(tolower(tostring(Message)))
    | extend InnerMessageParsed = parse_json(tolower(tostring(MessageParsed.message)))
    | extend Fields = bag_merge(MessageParsed, InnerMessageParsed)
    | extend Fields = bag_remove_keys(Fields, fn_filter)
    | extend Fields = bag_remove_keys(Fields, dynamic(['message']))
    | extend Fields = bag_merge(Fields, InnerMessageParsed.fields, MessageParsed.fields)
    | extend Fields = iff(Fields.correlationid != '00000000-0000-0000-0000-000000000000',
        Fields, bag_remove_keys(Fields, dynamic(['correlationid'])))
    | extend Fields = iff(Fields.name != '', Fields, bag_remove_keys(Fields, dynamic(['name'])))
    | extend Message = tostring(Fields)
    | extend Table = "uh";
let vmms = cluster('azcore.centralus').database('Fa').HyperVVmmsTable
    | where NodeId == fn_nodeId
    | where Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Message !contains "WHERE clause operator"
        and Message !contains "Provider could not handle query" // Annoying spam from LIKE query
    | where Level <= 4
    | extend Table = "vmms";
let vmwp = cluster('azcore.centralus').database('Fa').HyperVWorkerTable
    | where NodeId == fn_nodeId
    | where Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Level <= 4
    | extend MessageParsed = parse_json(tolower(tostring(Message)))
    | extend Fields = bag_remove_keys(MessageParsed, fn_filter)
    | extend Message = tostring(Fields)
    | extend Table = "vmwp";
let vmhv = cluster('azcore.centralus').database('Fa').HyperVHypervisorTable
    | where NodeId == fn_nodeId
    | where Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Level <= 4
    | extend Table = "vmhv";
let vpci = cluster('azcore.centralus').database('Fa').HyperVVPciTable
    | where NodeId == fn_nodeId
    | where Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Level <= 4
    | extend MessageParsed = parse_json(tolower(tostring(Message)))
    | extend Fields = bag_remove_keys(MessageParsed, fn_filter)
    | extend Message = tostring(Fields)
    | extend Table = "vpci";
union uh, vmms, vmwp, vmhv, vpci
| project
    PreciseTimeStamp,
    Table,
    Level,
    TaskName,
    Opcode,
    EventMessage = coalesce(EventMessage, Message),
    ActivityId,
    RelatedActivityId
```

**Key HyperVVmmsTable-specific filtering in this combined query:**
- Filters out `"WHERE clause operator"` and `"Provider could not handle query"` spam from LIKE queries
- Filters to `Level <= 4` (excludes verbose level 5)
- Uses `Message has fn_containerId` for container-scoped filtering
- Tags rows with `Table = "vmms"` for identification in the union

---

## 7. Common Investigation Patterns

### 7.1 Stop Container Failure

1. **Start** with the stop-container TSG query (Section 6.2) using `VmmsIndicateVmStateChange` and `TaskCompleted`.
2. Look for "Task started" with `TaskTypeName: "Turning Off"` and match its `TaskId` to `TaskCompleted`.
3. Check `TaskResultCode` — 0 = success, non-zero = failure.
4. Check `TaskElapsedTime` for reasonableness.
5. If VMMS shows success but HostAgent thinks container isn't stopped → route to **OneFleet Node/AzureHost-VMService**.
6. If VMMS shows an error → cross-reference with **HyperVWorkerTable** using the Pid from the worker process.
7. Check if host-initiated (EventId 18504) vs. guest-initiated (EventId 18508) shutdown using the HyperVEvents function.

### 7.2 Live Migration Failure

1. **Start** with the Common LM Query (Section 6.3) — unions VMMS + Worker + VID across source and destination nodes.
2. Tag each row as "Source" or "Destination" using `LMNode = iif(NodeId == fn_nodeIdSrc, "Source", "Destination")`.
3. Look for errors in VMMS and worker logs. Key keywords: `reset`, `failed`, `error`, `abort`.
4. For guest reset during LM: use Section 6.5 queries. Event 18514 in worker = guest reset.
5. For VdevOperation issues: use Section 6.6 query, check for stuck devices (Opcode 1 without matching Opcode 2).
6. For deeper investigation: use the Detailed LM Query (Section 6.4) which adds StorageStack, Hypervisor, and Analytic tables.

**HyperVVmmsTable's role in migration:**
> "Used for additional logging and errors that may have occurred during live migration."

### 7.3 Underhill Servicing

**Two types of servicing:**
- **Explicit** — VMMS creates `ReloadManagementVtlVmmsTaskDispatch` task
- **Implicit/Automatic** — VMMS creates `VmmsAutomaticManagementVtlReloadDispatch` task

**Investigation flow:**
1. **Check policy first** (Section 6.7) — is `ManagementVtlUpdatePolicy` blocking servicing?
2. **Check if servicing was attempted** (Section 6.8) during the impact window.
3. **Verify call reached worker** (Section 6.9) — look for `VmControl` + `ReloadManagementVtl` in union of VMMS + Worker.
4. **Check worker stages** — each stage logged with Opcode 1 (start) and Opcode 2 (end). Time between = stage duration. Missing Opcode 2 = stuck.
5. **Check worker result events:**
   - EventId **5124** = **failed** Underhill servicing (EventMessage shows stage, versions, correlation ID, error code)
   - EventId **5126** = **successful** servicing
6. **Dive into Underhill logs** using correlation ID from event 5126/5124 to trace operations in UnderhillEventTable.

**Servicing failure example (Event 5124 EventMessage):**
```
'17a902fc-6f35-4267-819e-81b8556518a2' failed to reload management VTL at stage: SaveManagementVtlState
with error code 0x800704C7 (%%2147943623).
(Virtual machine ID 6dccb78f-a243-46b4-8bd6-9cc2a556ca82).
Management VTL image file: 'vmfirmwareigvm.dll'.
Old management VTL image version: '1.4.83.0'.
New management VTL image version: [...]
```

**Internal servicing mechanism (VMMS path):**
1. Servicing call enters VMMS → checks if VM is Underhill, if loaded version < vmfirmwareigvm.dll version, if VM is Running.
2. If checks pass → task dispatched to worker process. If not → error returned.
3. Worker checks Underhill status and servicing policy.
4. Worker loads IGVM file, compares versions.
5. Worker instructs management VTL to save state.
6. **Point of no return** — any failure after state save requires VM reset.
7. Management VTL is reloaded with new firmware.

**Timeout behavior:** PilotFish has a 60-second timeout. Exceeding this sends cancellation → potential VM reset.

---

## 8. Cross-References to Other Tables

| Related Table | Relationship to HyperVVmmsTable |
|---|---|
| **HyperVWorkerTable** | Worker process (`vmwp.exe`) logs. VMMS dispatches tasks to worker; cross-reference by `Pid`, `Message` containing VmId/ContainerId, or `TaskName == "VmControl"`. Worker has detailed per-stage logging for servicing. |
| **HyperVVidTable** | VID driver events. Often unioned with VMMS + Worker for migration investigations. |
| **HyperVStorageStackTable** | Storage virtualization events. Used in detailed migration queries alongside VMMS. |
| **HyperVHypervisorTable** | Hypervisor events. Used in combined Underhill and detailed migration queries. |
| **HyperVVPciTable** | VPCI/device assignment events. Used in Underhill combined query. Important for diagnosing vmbus deadlocks. |
| **HyperVAnalyticEvents** | Additional analytic events. Included in detailed LM query. |
| **HyperVConfigTable** | VM configuration file events (vsconfig.dll, vmdatastore.dll). |
| **HyperVVmConfigSnapshot** | VM configuration snapshot. Used to check `IsUnderhill` flag and `SummaryType == "Configuration"`. |
| **UnderhillEventTable** | Guest VTL2 events. Different cluster: `wdgeventstore.kusto.windows.net` / `AzureHostOs`. Uses `VmName` (== ContainerId) for filtering. |
| **HyperVEvents** (SharedWorkspace function) | Aggregated Hyper-V events. Used for shutdown type detection (EventId 18504 = host, 18508 = guest). Located in SharedWorkspace database. |
| **LiveMigrationSessionCompleteLog** | LM session completion data. In `Fc` database. |
| **AirLiveMigrationEvents** | Detailed LM metrics (brownout, blackout, port programming delay). In `Air` database via Moseisley cluster. |
| **MycroftContainerSnapshot** | Container VM type, Trusted VM status, subscription IDs. In `AzureCP` database. |
| **MycroftContainerHealthSnapshot** | What control layers believe about container state. In `AzureCP` database. |
| **IfxOperationV2v1EtwTable** | HostAgent operation errors. Useful when diagnosing failed container stop/start. In `Fa` database. |
| **OsFileVersionTable** | File version info. Used to check vmfirmwareigvm.dll version for Underhill servicing. |

---

## 9. Important IDs and How They Relate

| ID | Description | Where Used |
|---|---|---|
| **NodeId** | Host node GUID | Primary filter on all queries |
| **ContainerId** | Container GUID (== `VmName` in Underhill) | Filter via `Message has fn_containerId` |
| **VmId** | Virtual Machine unique ID within Hyper-V | Filter via `Message has fn_vmId`. Different from ContainerId! |
| **VmUniqueId** | Azure-level VM resource ID (survives migrations) | Not directly in HyperVVmmsTable; use AzureCM tables |
| **TaskId** | VMMS task correlation ID | Found in `Message` JSON of `VmmsIndicateVmStateChange` and `TaskCompleted` |
| **ActivityId / RelatedActivityId** | ETW activity correlation | Used for tracing cross-process operations |
| **Correlation ID** | Underhill servicing correlation | Found in worker EventId 5124/5126 EventMessage; used to trace into UnderhillEventTable |

> **Caution:** Pay attention to the distinction between **VM ID** and **VM name == container ID** when using queries. See the [Underhill Kusto FAQ](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-kusto-queries-faq) for more information.

---

## 10. Tips and Gotchas

1. **Message field is JSON** — Use `parse_json(Message)` to extract structured fields. The payload varies by TaskName/EventId. Use `has` or `has_any` for GUID matching (faster than `contains` for GUIDs).

2. **EventMessage vs Message** — Some events populate `EventMessage`, others only populate `Message`. Always use the fallback pattern: `iif(isnotempty(EventMessage), EventMessage, Message)` or `coalesce(EventMessage, Message)`.

3. **Noisy events pre-filtered** — HyperVVmmsTable already has some noisy events filtered out at ingestion. For the combined Underhill query, you should also manually exclude `"WHERE clause operator"` and `"Provider could not handle query"` spam.

4. **Level filtering** — For combined queries, `Level <= 4` excludes verbose/informational (Level 5) to reduce noise. But for stop-container investigation, Level 5 events like `VmmsIndicateVmStateChange` are critical — don't filter them out!

5. **Opcode semantics** — Opcode 1 = Start, Opcode 2 = End. If you see Start without End for a task, it may be stuck. Time between Start and End = task duration.

6. **HResult interpretation for servicing:**
   - `ReloadManagementVtlVmmsTaskDispatch` stop event HResult 4096 = successful **dispatch only**, not overall success.
   - `VmmsAutomaticManagementVtlReloadDispatch` stop event `wilActivity_hresult = 0` = overall success.

7. **60-second PilotFish timeout** — If Underhill servicing exceeds this timeout, cancellation is sent. This may cause VM reset. Check for missing Opcode 2 events or look at timestamps.

8. **Point of no return in servicing** — After management VTL save state completes, any failure requires VM reset. This is visible in worker stage logging (correlation ID tracking).

9. **Migration: Source vs Destination** — Always tag nodes with `LMNode = iif(NodeId == fn_nodeIdSrc, "Source", "Destination")` in migration queries. Both nodes' VMMS logs are critical.

10. **ContainerId differs between source and destination** — In LM, the source and destination container IDs are different. Use `Message has_any(fn_containerIdSrc, fn_containerIdDest, fn_vmId)` to catch both.

11. **Not all LM logs have VM ID** — In a Live Migration, not all VMMS/Worker log entries contain a VM ID or Container ID. For comprehensive investigation, you may need to query without container filters and rely on time windows.

12. **Time windows vary by scenario:**
   - Stop container: ±1–1.3h from fault time
   - Migration: ±2h for container-filtered, ±25min for all logs (or wider for large VMs)
   - Underhill servicing: ±10min from impact time
   - Guest reset: ±10min from fault time

13. **Determining Underhill VMs** — Check `HyperVVmConfigSnapshot` with `SummaryType == "Configuration"` and look at `IsUnderhill` or `parse_json(SummaryJson).Settings.hcl.IsUnderhill`.

14. **SharedWorkspace functions** — The Hyper-V SME team maintains shared Kusto functions in the azcore cluster SharedWorkspace database. The `HyperVEvents()` function aggregates events from multiple Hyper-V tables.

---

## 11. Maintainer

- **Contact:** hypsme
- **IcM Queue:** RDOS/Azure Host OS SME - Virtualization (Hyper-V)
- **Routing:** For guest resets, route to WSD CFE\HCCompute-Guest OS Health (Windows) or LSG/Triage (Linux). See [aka.ms/rdosroute](https://aka.ms/rdosroute).
