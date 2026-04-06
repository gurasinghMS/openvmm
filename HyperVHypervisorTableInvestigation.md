# HyperV Table Investigation Guide

> Distilled from RDOS Livesite EngHub documentation.
> Primary table: **HyperVHypervisorTable** in `azcore.centralus.kusto.windows.net` / `Fa`
> Secondary tables: **HyperVVmConfigSnapshot**, **VmHealthRawStateEtwTable**, **WindowsEventsTable**

---

## Table of Contents

1. [HyperVHypervisorTable](#1-hypervhypervisortable)
2. [HyperVVmConfigSnapshot](#2-hypervvmconfigsnapshot)
3. [VmHealthRawStateEtwTable](#3-vmhealthrawstateetwtable)
4. [WindowsEventsTable](#4-windowseventstable)
5. [Cross-Table Investigation Patterns](#5-cross-table-investigation-patterns)
6. [Tips, Gotchas, and Known Issues](#6-tips-gotchas-and-known-issues)
7. [Related Hyper-V Kusto Tables Reference](#7-related-hyper-v-kusto-tables-reference)

---

## 1. HyperVHypervisorTable

### 1.1 Overview

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus.kusto.windows.net` |
| **Database** | `Fa` |
| **Source Process** | Hyper-V hypervisor binary — `hvix64.exe` (Intel), `hvax64.exe` (AMD x64), `hvaa64.exe` (ARM64) |
| **Provider Configuration** | Defined in `MdsFa.xml` |
| **What It Contains** | Events and traces from Hyper-V's hypervisor — the lowest software layer sitting above hardware, responsible for partition isolation, memory management, VP scheduling, and hardware arbitration |
| **Maintainer** | hypsme · IcM queue: `RDOS/Azure Host OS SME - Virtualization (Hyper-V)` |

The hypervisor is troubleshooted primarily in response to a host bugcheck when the crash bucket contains **HYPERVISOR_ERROR**. Spikes in reboot counts from specific hypervisor bugcheck codes (e.g., `HYPERVISOR_ERROR_458`) are auto-detected and reported via automation.

### 1.2 Key Fields

| Field | Type | Description |
|---|---|---|
| `PreciseTimeStamp` | datetime | High-resolution timestamp of the event |
| `NodeId` | string | Azure node identifier (GUID) |
| `Cluster` | string | Azure cluster name |
| `Region` | string | Azure region |
| `TaskName` | string | Logical grouping of the event (e.g., `"Vp config"`, hypervisor diagnostics) |
| `EventId` | int | Numeric event identifier |
| `Message` | string | JSON payload with event-specific data |
| `Opcode` | string | Operation code (Start, Stop, Info, etc.) |
| `Level` | int | Severity: 1=Critical, 2=Error, 3=Warning, 4=Info, 5=Verbose |
| `ProviderName` | string | ETW provider name |
| `Tid` | int | Thread ID |
| `ActivityId` | string | Correlation activity GUID |
| `RelatedActivityId` | string | Related correlation GUID |

### 1.3 Key TaskNames and Event Types

| TaskName | Description |
|---|---|
| `Vp config` | Processor feature capabilities of the node — contains `VpGuestProcessorFeatures_0`, `VpGuestProcessorFeatures_1`, `VpGuestProcessorXSaveFeatures` in the `Message` JSON |
| Hypervisor diagnostics | General diagnostic events emitted by the hypervisor including system log entries, configuration info, and crash data |

### 1.4 Kusto Query Examples

#### Basic Timeline Query for a Node

```kql
// Basic hypervisor event timeline for a node around a fault time
let fn_faultTime = datetime(2021-06-03T13:58:50.8635416);
let fn_startTime = fn_faultTime - 1d;
let fn_endTime = fn_startTime + 1d;
let fn_nodeId = "<paste node id here>";
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVHypervisorTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| project PreciseTimeStamp, TaskName, Message, Opcode
```

#### Query Processor Features ("Vp Config")

```kql
// View node processor feature capabilities from the hypervisor
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVHypervisorTable
| where PreciseTimeStamp between (ago(15d)..now())
| where TaskName == "Vp config"
| extend m = parse_json(Message)
| extend Bank0 = tolong(m.VpGuestProcessorFeatures_0)
| extend Bank1 = tolong(m.VpGuestProcessorFeatures_1)
| extend XSave = tolong(m.VpGuestProcessorXSaveFeatures)
| project PreciseTimeStamp, Cluster, NodeId, Bank0=tohex(Bank0), Bank1=tohex(Bank1), XSave
| take 1
```

#### Hypervisor Events for a Specific Container (Combined with ContainerId)

```kql
// Filter hypervisor events by containerId in Message payload
let fn_faultTime = datetime(2025-09-15 23:53:13);
let fn_startTime = fn_faultTime - 5m;
let fn_endTime = fn_faultTime + 1m;
let fn_nodeId = '94db1e7e-f598-4c19-ab60-a9f423a5e3ef';
let fn_containerId = '42390b9f-16fc-4761-8999-017175e7daf1';
cluster('azcore.centralus').database('Fa').HyperVHypervisorTable
| where NodeId == fn_nodeId
| where Message has fn_containerId
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where Level <= 4
| extend Table = "vmhv"
| project PreciseTimeStamp, TaskName, Opcode, Message, EventId, Level
```

#### VM-PHU HSR/HHR — Hypervisor Logs During Host Update

```kql
// For HSR (Hypervisor Soft Restart), Hypervisor Diagnostics logs are available
// for both the mature hypervisor and the proto hypervisor via HyperVHypervisorTable.
let fn_nodeId = "becebda6-2465-c52d-32e9-914307f3c327";
let fn_startTime = datetime(2025-08-28 22:14:12.1248301);
let fn_endTime = datetime(2025-08-28 22:45:25.2165597);
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVHypervisorTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| project PreciseTimeStamp, ProviderName, TaskName, Opcode, Message, Tid, EventId, Level
| sort by PreciseTimeStamp asc
```

> **Note:** For VM-PHU Self, non-hypervisor Hyper-V Kusto logs are ingested into a separate VM-PHU Trommel table, not the conventional Hyper-V tables. Hypervisor logs are the exception and remain in `HyperVHypervisorTable`.

#### Combined Multi-Table Query (Hypervisor + VMMS + Worker + VPCI + Underhill)

```kql
// Unified view across all Hyper-V tables for a container investigation
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
    | extend MessageParsed = parse_json(tolower(tostring(Message)))
    | extend InnerMessageParsed = parse_json(tolower(tostring(MessageParsed.message)))
    | extend Fields = bag_merge(MessageParsed, InnerMessageParsed)
    | extend Fields = bag_remove_keys(Fields, fn_filter)
    | extend Fields = bag_remove_keys(Fields, dynamic(['message']))
    | extend Fields = bag_merge(Fields, InnerMessageParsed.fields, MessageParsed.fields)
    | extend Message = tostring(Fields)
    | extend Table = "uh";
let vmms = cluster('azcore.centralus').database('Fa').HyperVVmmsTable
    | where NodeId == fn_nodeId
    | where Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Message !contains "WHERE clause operator"
        and Message !contains "Provider could not handle query"
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

### 1.5 Hypervisor Dump Debugging (for HYPERVISOR_ERROR Crashes)

When a host bugchecks with `HYPERVISOR_ERROR`:

1. **Collect dump**: On node via FcShell: `awdump.exe create live -hv` (the `-hv` flag is mandatory for hypervisor pages)
2. **Load debugger extension**: `.load hvexts.dll`
3. **Switch to hypervisor context**: `dx @$cursession.Hvx.CreateHvView()`
   - Prints `BugcheckOwner` = the processor that raised the error
4. **Load hotpatch symbols**: Inspect `KdpPatchSlot[0][1]` and `KdpPatchSlot[0][2]` for an active slot, then `.reload /f hv2=<StartVa>`
5. **View internal event log**: `dx @$cursession.Hvx.SysLog` — may report unresponsive processors, microcode status, etc.
6. **Get TRAP_FRAME**: Switch to bugcheck owner processor, find the thread running on it via `!hvthreads`, then `SetScope` and `kP`

### 1.6 Hypervisor SEL Events

On AH2021+ hardware, the hypervisor logs crash information to SEL:

```kql
let fn_nodeId = "b2e10058-8d24-f554-4971-940d58f99bb1";
let fn_faultTime = datetime(2023-04-27 01:13:09.2771333);
let fn_startTime = fn_faultTime - 30m;
let fn_endTime = fn_faultTime + 30m;
cluster('hawkeyedataexplorer.westus2.kusto.windows.net').database('HawkeyeLogs').
GetHypervisorSELEventsForNode(fn_nodeId, fn_startTime, fn_endTime)
```

---

## 2. HyperVVmConfigSnapshot

### 2.1 Overview

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus.kusto.windows.net` |
| **Database** | `Fa` |
| **Source** | Hyper-V Worker process (`vmwp.exe`) — periodic "VmSummary" telemetry |
| **What It Contains** | Point-in-time VM configuration snapshots — full JSON representation of a VM's settings, processor features, memory layout, HCL/Underhill state, device config, NUMA topology, and more |

> ⚠️ **CRITICAL CAVEAT**: `HyperVVmConfigSnapshot` is **only populated on hosts running AH2023 and newer**. Older host OSes do not log to this table. If you see zero rows, verify the host OS version first.

### 2.2 Key Fields

| Field | Type | Description |
|---|---|---|
| `PreciseTimeStamp` | datetime | When this snapshot was captured |
| `NodeId` | string | Azure node GUID |
| `ContainerId` | string | Container/VM name GUID |
| `SummaryType` | string | Type of summary — `"Configuration"` is the most common |
| `SummaryJson` | string | Full JSON blob with all VM configuration details |
| `IsUnderhill` | string | Whether this is an Underhill VM (`"true"` / `"false"`) — may be empty on some builds |
| `Cluster` | string | Azure cluster name |

### 2.3 SummaryJson Structure (Configuration type)

The `SummaryJson` field (when `SummaryType == "Configuration"`) is a rich JSON document containing:

```
├── Properties (global_id, type, version, name)
├── GlobalSettings (power, devices, storage_settings)
├── Security (tpm_enabled)
├── Settings
│   ├── global (logical_id)
│   ├── hcl (enabled, IsUnderhill)
│   ├── isolation (type: 0=None, 1=VBS, 2=...)
│   ├── memory (bank: size, dynamic_memory_enabled, backing_type)
│   ├── processors (count, hwthreads, features, ProcessorFeatureSet, EnlightenmentSet, cpu_group_id)
│   ├── vnuma (enabled)
│   └── topology (high_mmio_gap_mb, low_mmio_gap_mb)
├── Memory (Vtl2RamSizeInMb, Vtl2MmioSizeInMb, Vtl2RamBaseAddrOffsetMb, Vtl2MmioBaseAddrOffsetMb)
├── ManagementVtlState (CurrentFileName, CurrentFileVersion)
├── VmState (Current: VmStateRunning, VmStateStarting, etc.)
├── Resources (numa_mappings, Compute)
└── WorkerProcessSettings (SlpDataPath, MemoryDumpFilePath)
```

### 2.4 Kusto Query Examples

#### Determine if a VM is an Underhill VM

```kql
let fn_nodeId = "d71bdb10-080b-705a-ed75-568665161908";
let fn_containerId = "54eb2fa6-80e5-4ac4-82ac-ad3e19d160b2";
let fn_faultTime = datetime(2024-04-24T02:33:08Z);
let fn_startTime = fn_faultTime-1d;
let fn_endTime = fn_faultTime+1h;
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot
| where NodeId == fn_nodeId and ContainerId == fn_containerId
    and PreciseTimeStamp between(fn_startTime .. fn_endTime)
| where SummaryType == "Configuration"
| extend IsUnderhillFromJson = parse_json(SummaryJson).Settings.hcl.IsUnderhill
| project PreciseTimeStamp, IsUnderhill = iff(isnotempty(IsUnderhill), IsUnderhill, IsUnderhillFromJson)
| order by PreciseTimeStamp desc
| take 1
```

#### Get Underhill Version from VmConfigSnapshot

```kql
let fn_startTime = datetime(11-02-2023 07:35);
let fn_endTime = datetime(11-02-2023 21:35);
let fn_nodeId = "f30c2d3d-f286-a9e8-baa5-11fcf5e397af";
let fn_containerId = "c4c737e0-f408-40a7-9856-cec5d2085c3a";
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where ContainerId == fn_containerId
| where SummaryJson contains "vmfirmwareigvm"
| extend m = parse_json(SummaryJson)
| extend mem = parse_json(m.Memory)
| extend vtl = parse_json(m.ManagementVtlState)
| extend state = parse_json(m.VmState)
| project
    state.Current,
    vtl.CurrentFileName,
    vtl.CurrentFileVersion
```

#### Check VTL2 Memory Configuration

```kql
let fn_startTime = datetime(11-02-2023 07:35);
let fn_endTime = datetime(11-02-2023 21:35);
let fn_nodeId = "f30c2d3d-f286-a9e8-baa5-11fcf5e397af";
let fn_containerId = "c4c737e0-f408-40a7-9856-cec5d2085c3a";
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where ContainerId == fn_containerId
| where SummaryJson contains "Vtl2RamBaseAddrOffsetMb"
| extend m = parse_json(SummaryJson)
| extend mem = parse_json(m.Memory)
| extend state = parse_json(m.VmState)
| project
    state.Current,
    mem.Vtl2RamBaseAddrOffsetMb,
    mem.Vtl2RamSizeInMb,
    mem.Vtl2MmioBaseAddrOffsetMb,
    mem.Vtl2MmioSizeInMb
```

#### Find Underhill Clusters

```kql
// Identify clusters where Underhill is deployed
HyperVVmConfigSnapshot
| where PreciseTimeStamp > ago(7d) and IsUnderhill == 'true'
| summarize dcount(NodeId) by Cluster
```

#### Extract Processor Feature Set from VmConfigSnapshot

```kql
// The SummaryJson contains full ProcessorFeatureSet for the VM
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot
| where NodeId == "<nodeId>" and ContainerId == "<containerId>"
| where SummaryType == "Configuration"
| extend config = parse_json(SummaryJson)
| extend procFeatures = config.Settings.processors.ProcessorFeatureSet
| project PreciseTimeStamp,
    ProcessorFeatures = procFeatures.ProcessorFeatures,
    XsaveProcessorFeatures = procFeatures.XsaveProcessorFeatures,
    ProcessorFeatureSetMode = procFeatures.ProcessorFeatureSetMode
```

---

## 3. VmHealthRawStateEtwTable

### 3.1 Overview

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus.kusto.windows.net` |
| **Database** | `Fa` |
| **Source** | Host agents reporting Hyper-V's heartbeat Integrated Component (IC) status and VM power state |
| **What It Contains** | Periodic health state snapshots for running VMs — heartbeat state, power state, VSC operational status, and context of the last lifecycle operation |

This table is the primary place to check whether a VM is healthy from the host perspective. The heartbeat IC runs inside the guest and communicates back to the host. If the heartbeat is `HeartBeatStateOk`, the guest OS has booted and is responsive.

### 3.2 Key Fields

| Field | Type | Description |
|---|---|---|
| `PreciseTimeStamp` | datetime | When this health state was sampled |
| `ContainerId` | string | Container/VM name GUID |
| `NodeId` | string | Azure node GUID |
| `VmHyperVIcHeartbeat` | string | Heartbeat IC state (see states below) |
| `VmPowerState` | string | VM power state (see states below) |
| `HasHyperVHandshakeCompleted` | bool | Whether the Hyper-V management handshake with the guest completed |
| `IsVscStateOperational` | bool | Whether Virtual Service Clients are operational |
| `Context` | string | Last lifecycle operation context |

### 3.3 VmHyperVIcHeartbeat States

| State | Meaning | Investigation Action |
|---|---|---|
| `HeartBeatStateOk` | Guest OS is running and heartbeat IC is responding | VM is healthy — if customer reports issues, it is likely a guest-side or networking issue |
| `HeartBeatStateNoContact` | Heartbeat IC has not yet established contact | Normal during boot; if persistent, guest may be stuck in early boot or UEFI |
| `HeartBeatStateLostCommunication` | Heartbeat was previously OK but is now lost | Guest OS may have crashed, hung, or rebooted; check for bugchecks/triple faults |
| `NotMonitored` | VM is not being actively monitored | VM is stopped, saving, or in a transitional state |

### 3.4 VmPowerState Values

| State | Meaning |
|---|---|
| `PowerStateEnabled` | VM VPs are running |
| `NotMonitored` | VM is not in a running state |

### 3.5 Context Values

| Context | Meaning |
|---|---|
| `StartVm` | VM was most recently started |
| `StopVm` | VM was most recently stopped |
| `VirtualMachineRestarted` | VM was restarted (guest-initiated or host-initiated) |

### 3.6 Kusto Query Examples

#### VM Heartbeat Health Timeline (with change detection)

```kql
// Show health state changes for a container around a fault time
// This filters out consecutive identical states to show only transitions
let fn_startTime = datetime("2022-10-24T20:39:19.000Z");
let fn_endTime = datetime("2022-10-24T22:39:19.000Z");
let fn_containerId = "db1c16bf-2fb0-4082-b214-3896b8c53f11";
cluster('azcore.centralus').database('Fa').VmHealthRawStateEtwTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where ContainerId == fn_containerId
| project PreciseTimeStamp, ContainerId, VmHyperVIcHeartbeat, VmPowerState,
    HasHyperVHandshakeCompleted, IsVscStateOperational, Context
| sort by PreciseTimeStamp asc
| extend PrevTime = prev(PreciseTimeStamp)
| extend NextTime = next(PreciseTimeStamp)
| extend PrevContainer = prev(ContainerId)
| extend PrevHeartbeat = prev(VmHyperVIcHeartbeat)
| extend PrevPowerState = prev(VmPowerState)
| extend PrevHandshake = prev(HasHyperVHandshakeCompleted)
| extend PrevVscStateOperational = prev(IsVscStateOperational)
| extend PrevContext = prev(Context)
| where
    isnull(PrevTime) or
    isnull(NextTime) or
    (ContainerId != PrevContainer) or
    (VmHyperVIcHeartbeat != PrevHeartbeat) or
    (VmPowerState != PrevPowerState) or
    (HasHyperVHandshakeCompleted != PrevHandshake) or
    (IsVscStateOperational != PrevVscStateOperational) or
    (Context != PrevContext)
| project PreciseTimeStamp, ContainerId, VmHyperVIcHeartbeat, VmPowerState,
    HasHyperVHandshakeCompleted, IsVscStateOperational, Context
| extend level = case(VmHyperVIcHeartbeat == "HeartBeatStateOk", "info", "warning")
| order by PreciseTimeStamp desc
```

**Example output:**

| PreciseTimeStamp | ContainerId | VmHyperVIcHeartbeat | VmPowerState | HasHyperVHandshakeCompleted | IsVscStateOperational | Context | level |
|---|---|---|---|---|---|---|---|
| 2022-10-24T22:39:11Z | db1c16bf-... | HeartBeatStateOk | PowerStateEnabled | true | true | StartVm | info |
| 2022-10-24T21:39:48Z | db1c16bf-... | HeartBeatStateOk | PowerStateEnabled | true | true | StartVm | info |
| 2022-10-24T21:39:26Z | db1c16bf-... | HeartBeatStateNoContact | PowerStateEnabled | false | false | StartVm | warning |
| 2022-10-24T21:39:26Z | db1c16bf-... | NotMonitored | NotMonitored | false | false | StartVm | warning |
| 2022-10-24T21:38:14Z | db1c16bf-... | NotMonitored | NotMonitored | true | true | StopVm | warning |
| 2022-10-24T21:34:45Z | db1c16bf-... | HeartBeatStateLostCommunication | PowerStateEnabled | true | true | VirtualMachineRestarted | warning |
| 2022-10-24T20:39:30Z | db1c16bf-... | HeartBeatStateOk | PowerStateEnabled | true | true | VirtualMachineRestarted | info |

### 3.7 Interpreting Health State Transitions

A typical healthy VM lifecycle looks like:

```
NotMonitored → HeartBeatStateNoContact → HeartBeatStateOk
(stopped)      (starting/booting)        (guest OS up)
```

An unhealthy VM pattern:

```
HeartBeatStateOk → HeartBeatStateLostCommunication → (stays lost)
(was healthy)      (guest crashed/hung)
```

A reboot cycle:

```
HeartBeatStateOk → NotMonitored → HeartBeatStateNoContact → HeartBeatStateOk
(running)          (restarting)   (booting again)           (recovered)
```

---

## 4. WindowsEventsTable

### 4.1 Overview

| Property | Value |
|---|---|
| **Cluster** | `azcore.centralus.kusto.windows.net` |
| **Database** | `Fa` |
| **What It Contains** | Windows event log entries from Hyper-V event channels — Chipset, Worker, VMMS, and other Hyper-V providers |

> **Note:** This table is sometimes referred to as `WindowsEventTable` (no 's') in older documentation. Both refer to the same table.

### 4.2 Key Fields

| Field | Type | Description |
|---|---|---|
| `PreciseTimeStamp` | datetime | Event timestamp |
| `NodeId` | string | Azure node GUID |
| `EventId` | int | Windows event log Event ID |
| `Description` | string | Full event message text — contains ContainerId and VmId |
| `Level` | int | Severity level |
| `ProviderName` | string | ETW provider that generated the event |

### 4.3 Key Provider Names

| Provider | Description |
|---|---|
| `Microsoft-Windows-Hyper-V-Chipset` | UEFI/BIOS firmware events — boot success, boot failure, watchdog timeout |
| `Microsoft-Windows-Hyper-V-Worker` | Worker process events — VM lifecycle, guest crashes, triple faults, HCL errors |
| `Microsoft-Windows-Hyper-V-VMMS` | VM Management Service events |

### 4.4 Complete EventId Reference

#### Boot & Firmware Events (Provider: Microsoft-Windows-Hyper-V-Chipset)

| EventId | Name | Description |
|---|---|---|
| **18600** | UEFI Watchdog Timeout | `'<VmName>' has encountered a watchdog timeout and was reset.` The UEFI 2-minute boot watchdog (or 5-minute per-device watchdog) fired. Guest did not disable the timer in time. |
| **18601** | Successful Boot (Gen2) | `successfully booted an operating system.` — VM booted past firmware into guest OS. Only for Gen2 VMs. |
| **18602** | Guest Crash Dump Success | `has encountered a fatal error and a memory dump has been generated.` Error code 0x80 = NMI (often from UEFI watchdog). |
| **18603** | Boot Failure | Boot failure from firmware. Either VM is misconfigured (invalid boot disk) or guest storage corruption. |
| **18604** | Guest Crash Dump Failure | `has encountered a fatal error but a memory dump could not be generated.` |
| **18605** | Boot Failure (additional) | Additional boot failure event. |
| **18606** | INT19 Boot Attempt (Gen1, AH2023+) | Confirms that an INT19 transition to the boot loader was attempted for Gen1 VMs. Only available on AH2023+. |
| **18617** | New Workload Watchdog (AH2023+) | Same text as 18600 but is a different watchdog for workload liveness checks. This is a **guest OS issue** — route accordingly. |

#### VM Lifecycle Events (Provider: Microsoft-Windows-Hyper-V-Worker)

| EventId | Name | Description |
|---|---|---|
| **18500** | VM Started Successfully | `'<VmName>' started successfully. (Virtual machine ID <VmId>)` |
| **18514** | Guest Reset | `was reset by the guest operating system.` The guest OS itself initiated a reboot. |
| **18609** | VM Initialization | Logs processor features the VM will send to the hypervisor. Contains `ProcessorFeatures`, `ProcessorXsaveFeatures`, `PartitionCreationFlags`. |
| **18610** | HCL Fatal Error | Fatal virtual firmware error from the Host Compatibility Layer. Indicates HCL/Trusted Launch/CVM issue. |
| **18620** | VTL0 Start Failure (Underhill) | `MSVM_START_VTL0_REQUEST_ERROR` — Underhill VM's VTL0 failed to start. Check `ResultDocument` in Message JSON for the specific error. |

#### Guest Error Events (Provider: Microsoft-Windows-Hyper-V-Worker)

| EventId | Name | Description |
|---|---|---|
| **18539** | Triple Fault (General) | `MSVM_TRIPLE_FAULT_GENERAL_ERROR` — guest requested an unsupported operation causing a triple fault |
| **18540** | Triple Fault (Unsupported Feature) | `was reset because the guest operating system requested an operation that is not supported by Hyper-V.` |
| **18550** | Triple Fault (Invalid VP Register) | `was reset because an unrecoverable error occurred on a virtual processor that caused a triple fault.` — may be caused by hypervisor issue |
| **18560** | Triple Fault (Unrecoverable Exception) | `was reset because an unrecoverable error occurred on a virtual processor that caused a triple fault.` |
| **18590** | Guest Crash Report | `has encountered a fatal error. The guest operating system reported that it failed with the following error codes: ErrorCode0: 0x..., ErrorCode1: 0x...` |
| **18602** | Guest Crash Dump Generated | Guest OS generated a crash dump successfully |
| **18604** | Guest Crash Dump Failed | Guest OS attempted but failed to generate a crash dump |

### 4.5 Kusto Query Examples

#### Check UEFI Watchdog Timeout

```kql
let fn_faultTime = datetime(2022-10-07 02:09:00.0000000);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
let fn_nodeId = "3532d7cf-ae39-4a1d-bf6d-7b76f0970fd2";
let fn_containerId = "47e03d2b-2eae-4322-bd68-0a88b627af4a";
let fn_vmId = "1C940AD7-76A8-46C0-AA71-A7827B075C81";
cluster('azcore.centralus').database("Fa").WindowsEventTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    and EventId == 18600
    and ProviderName == "Microsoft-Windows-Hyper-V-Chipset"
    and (Description has fn_containerId or Description has fn_vmId)
| project PreciseTimeStamp, NodeId, Description, Level
```

#### Check for Guest Errors (Crashes, Triple Faults, Resets)

```kql
let fn_faultTime = datetime(2022-10-07 02:09:00.0000000);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
let fn_nodeId = "3532d7cf-ae39-4a1d-bf6d-7b76f0970fd2";
let fn_containerId = "47e03d2b-2eae-4322-bd68-0a88b627af4a";
let fn_vmId = "1C940AD7-76A8-46C0-AA71-A7827B075C81";
let fn_guestErrors = dynamic([
    18540, // MSVM_TRIPLE_FAULT_UNSUPPORTED_FEATURE_ERROR
    18550, // MSVM_TRIPLE_FAULT_INVALID_VP_REGISTER_ERROR
    18539, // MSVM_TRIPLE_FAULT_GENERAL_ERROR
    18560, // MSVM_TRIPLE_FAULT_UNRECOVERABLE_EXCEPTION_ERROR
    18590, // MSVM_GUEST_CRASH_REPORT
    18602, // MSVM_GUEST_CRASH_DUMP_SUCCESS
    18604, // MSVM_GUEST_CRASH_DUMP_FAILURE
    18514  // MSVM_GUEST_RESET_SUCCESS
]);
cluster('azcore.centralus').database("Fa").WindowsEventTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    and ProviderName == "Microsoft-Windows-Hyper-V-Worker"
    and EventId in (fn_guestErrors)
| where (Description has fn_containerId or Description has fn_vmId)
| project PreciseTimeStamp, NodeId, Description, Level, EventId
```

#### Check for HCL Fatal Firmware Error (Trusted Launch / CVM)

```kql
let fn_nodeId = "76fc09fb-4fc3-62b5-2232-06105a625ffa";
let fn_containerId = "d3ceec8d-8949-453f-ae1a-3631d624525f";
let fn_vmId = "DCF828EC-2850-43C9-8C20-463EA1D5F235";
cluster('azcore.centralus').database('Fa').WindowsEventTable
| where NodeId == fn_nodeId
| where Description contains fn_containerId or Description contains fn_vmId
| where ProviderName == "Microsoft-Windows-Hyper-V-Worker" and EventId == 18610
```

#### Check for Firmware Boot Events (Gen1 & Gen2)

```kql
let fn_nodeId = "bd90e0a3-8831-4ab9-8f45-0175d1f9bbe5";
let fn_containerId = "c28f795a-96b9-4195-909e-5e1e918bea5f";
let fn_startTime = ago(4d);
let fn_endTime = datetime(now);
let fn_eventIdArray = dynamic([18601, 18602, 18603, 18604, 18605, 18606]);
cluster('azcore.centralus').database('SharedWorkspace').HyperVEvents(fn_nodeId, fn_containerId, fn_startTime, fn_endTime)
| where EventId in (fn_eventIdArray)
```

#### Check if Guest Initiated a Reset (EventId 18514)

```kql
// Presence of EventId 18514 indicates the guest OS itself triggered the reset
let fn_faultTime = datetime(2024-05-01 1:00:00.000);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
let fn_nodeId = "ac13a952-9a4a-918c-437f-8ffb55a192aa";
let fn_containerId = "fa0f4c12-c406-41aa-8eae-275f2f5a638e";
cluster('azcore.centralus').database("Fa").HyperVWorkerTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime) and EventId == "18514"
| where NodeId == fn_nodeId
| where EventMessage has fn_containerId
| project PreciseTimeStamp, EventId, EventMessage
| sort by PreciseTimeStamp asc
```

### 4.6 UEFI Watchdog Timer Details

The UEFI firmware for most VMs is loaded by the worker process before VPs start. Timer sequence:

1. **Exit PEI phase** → starts a **2-minute** watchdog timer
2. **For each bootable device**:
   - Load image from device
   - If successful → start a **5-minute** timer
   - If failed → try next bootable device
3. **Guest OS is expected to disable the watchdog** when it boots successfully

If the watchdog fires, UEFI injects an **NMI** into the guest → causes crash → you may see both EventId 18600 (watchdog) and 18602 (crash dump with error code 0x80 = NMI).

---

## 5. Cross-Table Investigation Patterns

### 5.1 Pattern: "VM Started but Unhealthy"

**Step-by-step:**

1. **VmHealthRawStateEtwTable** → Check heartbeat timeline. Is `VmHyperVIcHeartbeat == "HeartBeatStateOk"` at the fault time?
   - If yes → guest is running, issue is guest-side or networking. Route via RDOS Route.
   - If `HeartBeatStateLostCommunication` → guest lost heartbeat after being healthy.
   - If `HeartBeatStateNoContact` persists → guest never completed boot.

2. **HyperVWorkerTable** → Check for "container started" event (EventId 18500) via `WindowsEventsTable` or `SharedWorkspace.HyperVContainerStarted()`.

3. **WindowsEventsTable** → Check boot events:
   - EventId 18601 (Gen2 successful boot)
   - EventId 18606 (Gen1 boot attempt, AH2023+)
   - EventId 18600 (UEFI watchdog timeout)
   - EventIds 18539/18540/18550/18560 (triple faults)
   - EventId 18590 (guest crash report)

4. **HyperVVmConfigSnapshot** → If Underhill, check `IsUnderhill`, VTL2 memory config, etc.

5. **HyperVHypervisorTable** → Check for hypervisor-level errors around the fault time.

### 5.2 Pattern: "Unexpected VM Reboot"

1. **HyperVWorkerTable** → Check for EventId 18514 (guest-initiated reset)
   - If present, something went wrong inside the guest after it started.
2. **VmHealthRawStateEtwTable** → Check timeline: how long after "container started" did the reset occur?
   - If > 5 minutes → likely guest OS issue
   - If < 5 minutes → may be boot/firmware issue
3. **WindowsEventsTable** → Check for triple faults (18539, 18540, 18550, 18560) and crash reports (18590, 18602)

### 5.3 Pattern: "HYPERVISOR_ERROR Bugcheck"

1. **HyperVHypervisorTable** → Timeline around the fault time
2. **Hawkeye** → `HawkeyeRCAEvents` for node fault RCA
3. **Hypervisor SEL** → `GetHypervisorSELEventsForNode()` on AH2021+
4. **Dump analysis** → Use hypervisor debugger extension (`hvexts.dll`)

### 5.4 Pattern: "Live Migration Failure"

1. **HyperVHypervisorTable** → Check processor features on both source and destination nodes via `TaskName == "Vp config"`
2. **HyperVVmConfigSnapshot** → Check VM's configured processor features via `ProcessorFeatureSet`
3. **HyperVWorkerTable** → EventId 18609 shows processor features the VM sent to the hypervisor
4. **Compare**: If VM's feature bit is set but destination node's feature is not → processor feature mismatch

### 5.5 Pattern: "Underhill VM Failure"

1. **HyperVVmConfigSnapshot** → Confirm `IsUnderhill == true` (AH2023+ only!)
2. **HyperVWorkerTable** → EventId 18620 (`MSVM_START_VTL0_REQUEST_ERROR`) — check `ResultDocument` for error details
3. **UnderhillEventTable** (in `wdgeventstore.kusto.windows.net/AzureHostOs`) → Check Underhill logs
4. **WindowsEventsTable** → EventId 18590 for guest crash events
5. **HyperVHypervisorTable** → Correlated hypervisor events

### 5.6 Pattern: "Node Fault — Is Hyper-V to Blame?"

1. **Hawkeye RCA**: `cluster('hawkeyedataexplorer.westus2').database('HawkeyeLogs').HawkeyeRCAEvents`
2. **HyperVHypervisorTable** → Any hypervisor errors around fault time
3. **WindowsEventsTable** → Guest errors that might have cascaded
4. **Host OS version**: `cluster('azcore.centralus').database('Fa').HostOsVersion` — was a recent update applied?

---

## 6. Tips, Gotchas, and Known Issues

### 6.1 General Tips

- **ContainerId vs VmId vs VmName**: In Azure, `ContainerId == VmName` (agent's ID). `VmId` is a Hyper-V internal construct that is different from ContainerId — the mapping is local to the node and is temporal. Always determine the VmId before querying Hyper-V tables that use it.
- **VmId mapping query**: Use `HyperVWorkerTable` with `TaskName == "VmNameToIdMapping"` to map ContainerId to VmId.
- **Sovereign Clouds**: If the incident mentions "Government", "Sovereign Cloud", "FairFax", or "Mooncake", standard Kusto queries may not work. Refer to the Sovereign Cloud Kusto Queries TSG.
- **SharedWorkspace Functions**: Many queries use `cluster('azcore.centralus').database('SharedWorkspace')` functions like `HyperVContainerStarted()`, `HyperVEvents()`, `AgentOperations()` — these are maintained by the Hyper-V SME v-team.
- **Log Retention**: Hyper-V logs in Kusto have retention limits (typically 60 days). If the VM start was long ago, logs may have aged out.

### 6.2 HyperVVmConfigSnapshot Gotchas

- ⚠️ **Only populated on AH2023+ hosts**. If you query this table and get zero rows, the host is running an older OS. In that case, check `UnderhillVersion` or ask the VMService team for config information.
- The `IsUnderhill` column may be empty on some builds — fall back to parsing `SummaryJson`: `parse_json(SummaryJson).Settings.hcl.IsUnderhill`.

### 6.3 WindowsEventsTable Gotchas

- EventId **18600** and **18617** log the same text but are different watchdogs. 18600 is the UEFI boot watchdog; 18617 (AH2023+) is a workload liveness watchdog and is a guest OS issue.
- Gen1 VMs do **not** have a successful boot event. Starting in AH2023, Gen1 VMs log EventId 18606 (INT19 boot attempt) but this only confirms the attempt, not success.
- The `Description` field contains the ContainerId/VmId in free text — use `has` operator for filtering, not `==`.

### 6.4 VmHealthRawStateEtwTable Gotchas

- `HeartBeatStateNoContact` immediately after VM start is **normal** — the guest hasn't booted yet. Only investigate if it persists for more than a few minutes.
- A VM can cycle through `NotMonitored → HeartBeatStateNoContact → HeartBeatStateOk` very quickly during normal start. Use the change-detection query (Section 3.6) to filter noise.

### 6.5 HyperVHypervisorTable Gotchas

- During VM-PHU Self, hypervisor logs **are** available in this table (unlike other Hyper-V tables which go to the Trommel table).
- The `Message` field is JSON but the schema varies by TaskName/EventId. Always use `parse_json(Message)` and inspect field names.
- For processor feature investigations, feature values are bitmasks that need to be compared bit-by-bit between source/destination nodes and the VM's configured features.

### 6.6 Known Benign Errors to Filter Out

| Table | Pattern | Reason |
|---|---|---|
| HyperVVmmsTable | `"WHERE clause operator"`, `"Provider could not handle query"` | WMI query spam — filter with `Message !contains` |
| UnderhillEventTable | `"Error: No information about IO-APIC in OF."` | Linux kernel in unfamiliar environment — benign |
| UnderhillEventTable | `"Cannot find an available gap in the 32-bit address range"` | Linux kernel — benign |
| UnderhillEventTable | `"PCI devices with unassigned 32-bit BARs may not work!"` | Linux kernel — benign |
| UnderhillEventTable | `"RETBleed: WARNING: Spectre v2 mitigation leaves CPU vulnerable..."` | Linux kernel — benign |
| UnderhillEventTable | `"PCI: Fatal: No config space access function found"` | Linux kernel — benign |

---

## 7. Related Hyper-V Kusto Tables Reference

All tables are in `cluster('azcore.centralus.kusto.windows.net').database('Fa')` unless noted.

| Table | Description |
|---|---|
| **HyperVTdprEvents** | Structured TDPR-style timeline events based on `HyperV.Regions.xml` regions of interest |
| **HyperVWorkerTable** | Events from `vmwp.exe` (worker process). Message in JSON. Excludes EventId 23100–23145 (virtual device data). |
| **HyperVVmmsTable** | Events from `vmms.exe` (VM Management Service). Filtered to exclude noisy events. |
| **HyperVComputeTable** | Select events from `vmcompute.exe` (Host Compute Service). Narrow collection due to HCS noisiness. |
| **HyperVConfigTable** | Events from VM configuration/runtime file management (`vsconfig.dll`, `vmdatastore.dll`) |
| **HyperVHypervisorTable** | Events from the hypervisor (`hvix64.exe` / `hvax64.exe` / `hvaa64.exe`) |
| **HyperVVidTable** | Filtered events from VID (`vid.sys`/`vid.dll`) — hypervisor communication interface |
| **HyperVStorageStackTable** | Filtered storage virtualization provider events. Payload as JSON in `Message`. |
| **HyperVVPciTable** | Events for VPCI and device assignment stack. Data in JSON `Message` field. |
| **UnderhillEventTable** | Events from guest VTL2 (Underhill). Located in `wdgeventstore.kusto.windows.net/AzureHostOs`. |
| **HyperVVmConfigSnapshot** | Point-in-time VM configuration snapshots. AH2023+ only. |
| **VmHealthRawStateEtwTable** | VM heartbeat and health state snapshots |

### Other Useful Tables (Different Clusters/Databases)

| Table | Cluster / Database | Description |
|---|---|---|
| `HawkeyeRCAEvents` | `hawkeyedataexplorer.westus2.kusto.windows.net` / `HawkeyeLogs` | Automated RCA for node faults |
| `MycroftContainerHealthSnapshot` | `azcore.centralus.kusto.windows.net` / `AzureCP` | Control layer view of container state |
| `MycroftContainerSnapshot` | `azcore.centralus.kusto.windows.net` / `AzureCP` | Container metadata (VM type, TL, CVM, subscription) |
| `LogContainerSnapshot` | `gandalf.kusto.windows.net` / `AzureCM` | Container lifecycle data (Gen2, properties, etc.) |
| `HostOsVersion` | `azcore.centralus.kusto.windows.net` / `Fa` | Node OS version history |
| `IfxOperationV2v1EtwTable` | `azcore.centralus.kusto.windows.net` / `Fa` | Host agent operation results |
| `GuestOSDetailEtwTable` | `azcore.centralus.kusto.windows.net` / `Fa` | Guest OS type (Linux vs Windows) |
| `OsFileVersionTable` | `azcore.centralus.kusto.windows.net` / `Fa` | Binary file versions on the host (e.g., `vmfirmwareigvm.dll`) |
| `LiveMigrationSessionCompleteLog` | `azcore.centralus.kusto.windows.net` / `Fc` | Live migration session data |
| `AirLiveMigrationEvents` | Various | Detailed LM session info (brownout, blackout times) |
| `VmServiceContainerOperations` | `azcore.centralus.kusto.windows.net` / `Fa` | VM service operations including Underhill flags |
| `KaHostSummary` | `wdgeventstore.kusto.windows.net` / `KernelAgent` | Memory/memory partition info for diagnosing fragmentation |

### Required Access

| Security Group | Access Granted |
|---|---|
| VMPHU Kusto Viewer SG | VMPHU Kusto tables |
| AzDeployer Kusto User SG | AzDeployer/OaaS Kusto |
| AlbusViewer SG | gandalffollower.centralus cluster |
| HostOsData Kusto Viewers SG | hostosdata.centralus.kusto.windows.net |
| XLivesiteKustoAccess | xstore.kusto.windows.net |
| IcM-Kusto-Access | icmcluster.kusto.windows.net |
| AME\TM-HVX | Access to decrypted hypervisor dumps in AzureWatson |

---

*Last updated from RDOS Livesite EngHub documentation. Source TSGs: hyperv-kusto-queries, hypervisor-overview, running-container-unhealthy, underhill-tsg, underhill-kusto-queries-faq, vmphu, hyperv-processor-features, migration-failure, stop-container-failure.*
