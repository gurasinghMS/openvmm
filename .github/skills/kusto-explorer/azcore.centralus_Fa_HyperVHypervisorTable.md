# HyperVHypervisorTable

**Type:** Table  
**Cluster:** `https://azcore.centralus.kusto.windows.net`  
**Database:** `Fa`  
**Full Path:** `azcore.centralus.kusto.windows.net` → `Fa` → `HyperVHypervisorTable`

---

## 1. Description

The HyperVHypervisorTable captures **events and traces from Hyper-V's hypervisor** — the lowest software layer sitting above hardware, responsible for partition isolation, memory management, virtual processor (VP) scheduling, and hardware arbitration.

**Key Characteristics:**
- **Data Source:** ETW events from Hyper-V hypervisor binaries — `hvix64.exe` (Intel), `hvax64.exe` (AMD x64), `hvaa64.exe` (ARM64)
- **Provider Configuration:** Defined in `MdsFa.xml`
- **Volume:** Very high-throughput (~968 million events per day across all regions), dominated by Verbose (Level 5) and Info (Level 4) events
- **Retention:** Typically ~60 days of historical data
- **Severity Coverage:** Captures all log levels — Verbose (5), Info (4), Warning (3), Error (2)
- **Primary Use Case:** Troubleshooting host bugchecks with `HYPERVISOR_ERROR` crash buckets, investigating partition lifecycle, device assignment, processor feature compatibility, and hypervisor soft restart (HSR) events
- **Maintainer:** hypsme · IcM queue: `RDOS/Azure Host OS SME - Virtualization (Hyper-V)`

**Provider Names:**
| Provider | Description |
|----------|-------------|
| `Microsoft-Windows-Hyper-V-Hypervisor` | Core hypervisor operational events (partition create/delete, errors). Uses `ChannelName` and `EventMessage`. |
| `Microsoft.Windows.HyperV.Hypervisor.Diagnostics` | Diagnostics/telemetry events (device attach/detach, config, debug messages). Uses `TaskName` and JSON `Message`. |
| _(empty)_ | Some events have no provider name set |

**Channel Names:**
| Channel | Volume | Description |
|---------|--------|-------------|
| `Microsoft-Windows-Hyper-V-Hypervisor-Operational` | ~254M/day | Primary operational events (partition lifecycle, etc.) |
| `Microsoft-Windows-Hyper-V-Hypervisor-Admin` | ~368K/day | Administrative/error events |
| `System` | ~1K/day | System-level hypervisor events |
| _(empty)_ | ~714M/day | Diagnostics provider events (no channel) |

**Message Format:**
The `Message` field contains JSON payloads whose schema varies by `TaskName`/`EventId`. Always use `parse_json(Message)` and inspect field names. The `EventMessage` field contains human-readable text for core hypervisor events (e.g., "Hyper-V successfully created a new partition").

---

## 2. Schema

| Column Name | Type | Description |
|-------------|------|-------------|
| `TIMESTAMP` | datetime | Event ingestion timestamp in UTC. **Do not use for queries** — use `PreciseTimeStamp` instead |
| `PreciseTimeStamp` | datetime | **Precise event timestamp in UTC.** Use for ALL time-based queries and retention boundaries |
| `Environment` | string | Azure environment identifier (e.g., `"PROD"`) |
| `Region` | string | Azure region (e.g., `"centralus"`, `"eastus2"`) |
| `DataCenter` | string | Physical datacenter identifier (e.g., `"DSZ04"`, `"CDM12"`) |
| `Cluster` | string | Azure cluster name (e.g., `"DSZ04PrdApp25"`) |
| `NodeIdentity` | string | IP address of the physical compute node |
| `NodeId` | string | **Unique GUID for the physical compute node.** Primary infrastructure filter |
| `DeviceId` | string | Device identifier for the host component generating the event |
| `Level` | long | **ETW severity level:** 1=Critical, 2=Error, 3=Warning, 4=Informational, 5=Verbose |
| `ProviderGuid` | string | ETW provider GUID |
| `ProviderName` | string | ETW provider name (see Provider Names above) |
| `EventId` | long | Numeric event identifier. `0` = diagnostics trace events; `16641`/`16642` = partition create/delete |
| `Pid` | long | Process ID (typically `4` = System for hypervisor events) |
| `Tid` | long | Thread ID |
| `OpcodeName` | string | ETW opcode name (e.g., `"Info"`, often empty) |
| `KeywordName` | string | ETW keyword name (typically empty) |
| `TaskName` | string | **Logical grouping of the event.** Critical for filtering diagnostics events (see Section 4) |
| `ChannelName` | string | Windows event log channel name |
| `EventMessage` | string | Human-readable event message (populated for core hypervisor events like partition create/delete) |
| `ActivityId` | string | Correlation activity GUID |
| `Task` | long | Numeric task identifier |
| `Opcode` | long | Numeric opcode |
| `RelatedActivityId` | string | Related correlation GUID |
| `Message` | string | **JSON payload with event-specific data.** Schema varies by TaskName/EventId — always use `parse_json(Message)` |
| `__AuthType__` | string | Authentication type (internal) |
| `__AuthIdentity__` | string | Authentication identity (internal) |
| `SourceNamespace` | string | Source namespace (e.g., `"Fa"`) |
| `SourceMoniker` | string | Source moniker (e.g., `"FaDiagdm03"`) |
| `SourceVersion` | string | Source version (e.g., `"Ver249v0"`) |
| `ObfuscatedData` | string | Obfuscated data field (typically empty) |
| `AutopilotEnvironment` | string | Autopilot environment identifier, includes cluster and region info |

---

## 3. Critical Column Guide — What to Query By

### Tier 1: Always Include (Performance Critical)
These columns are indexed and should always be part of your `where` clause:

| Column | Why | Example |
|--------|-----|---------|
| `PreciseTimeStamp` | **Time-scoping is mandatory** for performance. Always narrow to the smallest window possible. | `where PreciseTimeStamp between (fn_startTime .. fn_endTime)` |
| `NodeId` | **Primary node filter.** Every investigation starts with a specific node. | `where NodeId == "61cea02f-f54a-d5d5-100f-3c7adb82692b"` |

### Tier 2: Strongly Recommended (Narrows Results)

| Column | Why | Example |
|--------|-----|---------|
| `Level` | Filter by severity. Use `Level <= 2` for errors, `Level <= 3` to include warnings. | `where Level <= 3` |
| `TaskName` | **Critical for diagnostics events.** Groups events by logical function. | `where TaskName == "Vp config"` |
| `EventId` | Filter specific event types. `16641` = partition created, `16642` = partition deleted. | `where EventId == 16641` |
| `ProviderName` | Distinguish core hypervisor vs diagnostics events. | `where ProviderName == "Microsoft-Windows-Hyper-V-Hypervisor"` |

### Tier 3: Post-Filter (Refinement)

| Column | Why | Example |
|--------|-----|---------|
| `Message` | Search for specific ContainerIds, PartitionIds, or error details in JSON payload. Use `has` operator. | `where Message has fn_containerId` |
| `EventMessage` | Search human-readable event text. | `where EventMessage has "partition"` |
| `Cluster` | Filter by Azure cluster. | `where Cluster == "DSZ04PrdApp25"` |
| `Region` | Filter by Azure region. | `where Region == "centralus"` |
| `ChannelName` | Filter by event channel. | `where ChannelName == "Microsoft-Windows-Hyper-V-Hypervisor-Admin"` |

---

## 4. Key TaskNames Reference

TaskNames are the primary way to categorize diagnostics events (from `Microsoft.Windows.HyperV.Hypervisor.Diagnostics` provider). The `EventId` is `0` for most diagnostics events; use `TaskName` to distinguish them.

| TaskName | Daily Volume | Description | Message JSON Fields |
|----------|-------------|-------------|---------------------|
| _(empty)_ | ~263M | Core hypervisor events (partition create/delete, etc.). Use `EventId` to distinguish. | `{"PartitionId": <int>}` |
| `Device Detached` | ~223M | Device removed from a partition | `{"Partition": <int>, "DeviceType": <int>, "Device": <int>, "DevicePath": ""}` |
| `Device Attached` | ~169M | Device assigned to a partition | `{"Partition": <int>, "DeviceType": <int>, "Device": <int>, "DevicePath": "", "Flags": <int>, "LogicalId": <int>}` |
| `Device implicitly Attached` | ~75M | Device implicitly assigned (e.g., during partition creation) | Similar to Device Attached |
| `Device Attached To Default Domain` | ~68M | Device assigned to the default IOMMU domain | Similar to Device Attached |
| `Device explicitly Attached` | ~49M | Device explicitly assigned (e.g., GPU passthrough) | Similar to Device Attached |
| `HvldrDebugMsg` | ~22M | Hypervisor loader debug messages | Debug text |
| `Cpu group` | ~13M | CPU group configuration changes | CPU group details |
| `Processor Feature Update` | ~12M | Processor feature bitmask updates | Feature bitmask data |
| `Device implicitly Detached` | ~7M | Device implicitly removed | Similar to Device Detached |
| `Ke config` | ~5M | Kernel configuration information | Config data |
| `Hyp version` | ~5M | Hypervisor version information | Version strings |
| `Hypervisor hotpatch state` | ~5M | Hotpatch status (applied, pending, etc.) | Hotpatch state data |
| `Hal config` | ~5M | HAL configuration information | Config data |
| `Th config` | ~5M | Thread configuration information | Config data |
| `Val config` | ~5M | Validation configuration | Config data |
| `Vp config` | ~5M | **Processor feature capabilities of the node** — contains `VpGuestProcessorFeatures_0`, `VpGuestProcessorFeatures_1`, `VpGuestProcessorXSaveFeatures` in the Message JSON | `{"VpGuestProcessorFeatures_0": <int>, "VpGuestProcessorFeatures_1": <int>, "VpGuestProcessorXSaveFeatures": <int>}` |
| `Iommu config` | ~5M | IOMMU configuration details | Config data |
| `Timer trace: physical delayed` | ~4M | Physical timer delay diagnostics | Timer data |
| `Shutdown` | ~4M | Hypervisor shutdown events | Shutdown details |
| `HvldrFailureLocation` | varies | Hypervisor loader failure location (Level 2 = Error) | `{"BaseLocation": "<string>", "Line": <int>, "BalStatus": <int>, "AuxData": <int>}` |

---

## 5. Key Event IDs Reference

Events with `EventId != 0` come from the `Microsoft-Windows-Hyper-V-Hypervisor` provider and have human-readable `EventMessage` text.

| EventId | Daily Volume | EventMessage Pattern | Description |
|---------|-------------|----------------------|-------------|
| `0` | ~679M | _(none — use TaskName)_ | Diagnostics trace events from `Microsoft.Windows.HyperV.Hypervisor.Diagnostics`. Use `TaskName` to categorize. |
| `16641` | ~127M | `"Hyper-V successfully created a new partition (partition <N>)."` | Partition creation event. Message JSON: `{"PartitionId": <N>}` |
| `16642` | ~126M | `"Hyper-V successfully deleted a partition (partition <N>)."` | Partition deletion event. Message JSON: `{"PartitionId": <N>}` |
| `12329` | ~28M | _(varies)_ | Hypervisor diagnostic event |
| `12311` | ~650K | _(varies)_ | Hypervisor diagnostic event |
| `12324` | ~587K | _(varies)_ | Hypervisor diagnostic event |
| `12325` | ~587K | _(varies)_ | Hypervisor diagnostic event |
| `12320` | ~389K | _(varies)_ | Hypervisor diagnostic event |
| `12321` | ~389K | _(varies)_ | Hypervisor diagnostic event |
| `12323` | ~389K | _(varies)_ | Hypervisor diagnostic event |
| `12322` | ~389K | _(varies)_ | Hypervisor diagnostic event |
| `12317` | ~372K | _(varies)_ | Hypervisor loader failure location (`HvldrFailureLocation`) |
| `12` | ~360K | _(varies)_ | Hypervisor diagnostic event |
| `12327` | ~289K | _(varies)_ | Hypervisor diagnostic event |
| `12550` | ~262K | _(varies)_ | Hypervisor diagnostic event |
| `12297` | ~254K | _(varies)_ | Hypervisor diagnostic event |
| `12304` | ~207K | _(varies)_ | Hypervisor diagnostic event |
| `12310` | ~207K | _(varies)_ | Hypervisor diagnostic event |
| `39` | ~207K | _(varies)_ | Hypervisor diagnostic event |
| `156` | ~207K | _(varies)_ | Hypervisor diagnostic event |

---

## 6. Common Message Patterns

### 6.1 Partition Lifecycle Messages
```json
// EventId 16641 — Partition Created
{"PartitionId": 5300}
// EventMessage: "Hyper-V successfully created a new partition (partition 5300)."

// EventId 16642 — Partition Deleted
{"PartitionId": 5299}
// EventMessage: "Hyper-V successfully deleted a partition (partition 5299)."
```

### 6.2 Device Attach/Detach Messages
```json
// TaskName: "Device Attached"
{"Partition": 5314, "DeviceType": 1, "Device": 5531917877248, "DevicePath": "", "Flags": 9, "LogicalId": 2725465152}

// TaskName: "Device Detached"
{"Partition": 5402, "DeviceType": 1, "Device": 5514738008064, "DevicePath": ""}
```

### 6.3 Processor Feature Configuration (Vp config)
```json
// TaskName: "Vp config"
{"VpGuestProcessorFeatures_0": <bitmask>, "VpGuestProcessorFeatures_1": <bitmask>, "VpGuestProcessorXSaveFeatures": <bitmask>}
```
These are bitmask values representing processor capabilities. Compare bit-by-bit between source and destination nodes for live migration compatibility analysis.

### 6.4 Hypervisor Loader Failure
```json
// TaskName: "HvldrFailureLocation", Level: 2 (Error)
{"BaseLocation": "Microcode", "Line": 238, "BalStatus": 9223372040076001467, "AuxData": 0}
```
Indicates a hypervisor loader failure, often related to microcode loading issues.

---

## 7. Level Distribution

| Level | Name | Daily Volume (~) | Description |
|-------|------|------------------|-------------|
| 5 | Verbose | ~680M | Diagnostics trace events (device attach/detach, config events) |
| 4 | Informational | ~288M | Operational events (partition create/delete) |
| 3 | Warning | ~463K | Warning conditions |
| 2 | Error | ~381K | Error events (loader failures, etc.) |

> **Note:** Level 1 (Critical) events were not observed in the last 24 hours but may occur during severe hypervisor faults.

---

## 8. Sample Queries

### 8.1 Basic Timeline Query for a Node

```kql
// Hypervisor event timeline for a node around a fault time
let fn_faultTime = datetime(2025-06-03T13:58:50Z);
let fn_startTime = fn_faultTime - 1d;
let fn_endTime = fn_startTime + 1d;
let fn_nodeId = "<paste node id here>";
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVHypervisorTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| project PreciseTimeStamp, TaskName, Message, Opcode
```

### 8.2 Hypervisor Events for a Specific Container

```kql
// Filter hypervisor events by containerId in Message payload
let fn_faultTime = datetime(2025-09-15 23:53:13);
let fn_startTime = fn_faultTime - 5m;
let fn_endTime = fn_faultTime + 1m;
let fn_nodeId = '<paste node id here>';
let fn_containerId = '<paste container id here>';
cluster('azcore.centralus').database('Fa').HyperVHypervisorTable
| where NodeId == fn_nodeId
| where Message has fn_containerId
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where Level <= 4
| extend Table = "vmhv"
| project PreciseTimeStamp, TaskName, Opcode, Message, EventId, Level
```

### 8.3 Query Processor Features ("Vp Config")

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

### 8.4 Error Events Investigation

```kql
// Check for errors on a specific node
let fn_nodeId = "<paste node id here>";
let fn_startTime = ago(1d);
let fn_endTime = now();
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVHypervisorTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where Level <= 2
| project PreciseTimeStamp, Level, TaskName, EventId, EventMessage, Message
| order by PreciseTimeStamp desc
```

### 8.5 Hypervisor Logs During Host Update (HSR/HHR)

```kql
// For HSR (Hypervisor Soft Restart), Hypervisor Diagnostics logs are available
// for both the mature hypervisor and the proto hypervisor
let fn_nodeId = "<paste node id here>";
let fn_startTime = datetime(2025-08-28 22:14:12);
let fn_endTime = datetime(2025-08-28 22:45:25);
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVHypervisorTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| project PreciseTimeStamp, ProviderName, TaskName, Opcode, Message, Tid, EventId, Level
| sort by PreciseTimeStamp asc
```

> **Note:** During VM-PHU Self, hypervisor logs **are** available in this table (unlike other Hyper-V tables which go to the VM-PHU Trommel table). Hypervisor logs are the exception and remain in `HyperVHypervisorTable`.

### 8.6 Partition Lifecycle Summary for a Node

```kql
// Count partition creates and deletes for a node over time
let fn_nodeId = "<paste node id here>";
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVHypervisorTable
| where PreciseTimeStamp > ago(1d)
| where NodeId == fn_nodeId
| where EventId in (16641, 16642)
| summarize
    Created = countif(EventId == 16641),
    Deleted = countif(EventId == 16642)
    by bin(PreciseTimeStamp, 1h)
| order by PreciseTimeStamp asc
```

### 8.7 Fleet-Wide Error Analysis

```kql
// Find clusters with the most hypervisor errors in last 24h
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVHypervisorTable
| where PreciseTimeStamp > ago(1d)
| where Level <= 2
| summarize ErrorCount = count() by Cluster, TaskName
| order by ErrorCount desc
| take 20
```

### 8.8 Hypervisor SEL Events (AH2021+ Hardware)

```kql
// On AH2021+ hardware, the hypervisor logs crash information to SEL
let fn_nodeId = "<paste node id here>";
let fn_faultTime = datetime(2023-04-27 01:13:09);
let fn_startTime = fn_faultTime - 30m;
let fn_endTime = fn_faultTime + 30m;
cluster('hawkeyedataexplorer.westus2.kusto.windows.net').database('HawkeyeLogs').
GetHypervisorSELEventsForNode(fn_nodeId, fn_startTime, fn_endTime)
```

---

## 9. Cross-Table Query Patterns

### 9.1 ID Relationships

| ID | Where It Appears | How to Map |
|----|-------------------|-----------|
| `NodeId` | All Hyper-V tables, UnderhillEventTable | Direct join on `NodeId` |
| `ContainerId` / `VmName` | Not a direct column — search in `Message` field using `has` operator | `where Message has fn_containerId` |
| `PartitionId` | In `Message` JSON for EventId 16641/16642 | `parse_json(Message).PartitionId` |
| `VmId` | Not in this table. Map via `HyperVWorkerTable` `TaskName == "VmNameToIdMapping"` | Cross-table lookup |

> **Important:** `ContainerId == VmName` (the agent's ID). `VmId` is a Hyper-V internal construct that differs from ContainerId — the mapping is local to the node and temporal. Use `HyperVWorkerTable` with `TaskName == "VmNameToIdMapping"` to resolve.

### 9.2 Combined Multi-Table Query (Hypervisor + VMMS + Worker + VPCI + Underhill)

```kql
// Unified view across all Hyper-V tables for a container investigation
let fn_nodeId = '<paste node id here>';
let fn_containerId = '<paste container id here>';
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

### 9.3 Live Migration Processor Feature Comparison

```kql
// Compare processor features between source and destination nodes
// Step 1: Get source node features from HyperVHypervisorTable
let fn_sourceNodeId = "<source node id>";
let fn_destNodeId = "<dest node id>";
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVHypervisorTable
| where PreciseTimeStamp > ago(7d)
| where NodeId in (fn_sourceNodeId, fn_destNodeId)
| where TaskName == "Vp config"
| extend m = parse_json(Message)
| extend Bank0 = tolong(m.VpGuestProcessorFeatures_0)
| extend Bank1 = tolong(m.VpGuestProcessorFeatures_1)
| extend XSave = tolong(m.VpGuestProcessorXSaveFeatures)
| summarize arg_max(PreciseTimeStamp, *) by NodeId
| project NodeId, Bank0=tohex(Bank0), Bank1=tohex(Bank1), XSave=tohex(XSave)
```

Then compare with the VM's configured features:
```kql
// Step 2: Get VM's configured processor features from HyperVVmConfigSnapshot
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot
| where NodeId == "<source node id>" and ContainerId == "<containerId>"
| where SummaryType == "Configuration"
| extend config = parse_json(SummaryJson)
| extend procFeatures = config.Settings.processors.ProcessorFeatureSet
| project PreciseTimeStamp,
    ProcessorFeatures = procFeatures.ProcessorFeatures,
    XsaveProcessorFeatures = procFeatures.XsaveProcessorFeatures,
    ProcessorFeatureSetMode = procFeatures.ProcessorFeatureSetMode
```

```kql
// Step 3: Get VM's initialized processor features from HyperVWorkerTable
// EventId 18609 shows processor features the VM sent to the hypervisor
cluster('azcore.centralus').database('Fa').HyperVWorkerTable
| where NodeId == "<source node id>"
| where EventId == 18609
| where Message has "<containerId>"
| project PreciseTimeStamp, Message
```

If the VM's feature bit is set but the destination node's feature is not → **processor feature mismatch** is the likely cause of migration failure.

### 9.4 Related Tables Quick Reference

All tables are in `cluster('azcore.centralus.kusto.windows.net').database('Fa')` unless noted.

| Table | Description | Join Key |
|-------|-------------|----------|
| `HyperVWorkerTable` | Events from `vmwp.exe` (worker process) | `NodeId`, `Message has ContainerId` |
| `HyperVVmmsTable` | Events from `vmms.exe` (VM Management Service) | `NodeId`, `Message has ContainerId` |
| `HyperVVPciTable` | VPCI and device assignment events | `NodeId`, `Message has ContainerId` |
| `HyperVStorageStackTable` | Storage virtualization events | `NodeId` |
| `HyperVComputeTable` | Host Compute Service events | `NodeId` |
| `HyperVConfigTable` | VM configuration/runtime file events | `NodeId` |
| `HyperVVidTable` | VID (hypervisor communication interface) events | `NodeId` |
| `HyperVVmConfigSnapshot` | VM configuration snapshots (AH2023+ only) | `NodeId`, `ContainerId` |
| `VmHealthRawStateEtwTable` | VM heartbeat and health state | `NodeId`, `ContainerId` |
| `WindowsEventsTable` | Windows event log (Chipset, Worker, VMMS providers) | `NodeId`, `Description has ContainerId` |
| `UnderhillEventTable` | Underhill/OpenHCL events (`wdgeventstore.kusto.windows.net/AzureHostOs`) | `NodeId`, `VmName == ContainerId` |
| `HawkeyeRCAEvents` | Node fault RCA (`hawkeyedataexplorer.westus2.kusto.windows.net/HawkeyeLogs`) | `NodeId` |
| `HostOsVersion` | Host OS version history | `NodeId` |

---

## 10. Investigation Playbook

### 10.1 HYPERVISOR_ERROR Bugcheck Investigation

This is the **primary reason** to query this table. A host bugcheck with `HYPERVISOR_ERROR` means the hypervisor itself crashed.

1. **Get the fault time and NodeId** from the incident
2. **Query HyperVHypervisorTable** for all events around the fault time:
   ```kql
   cluster('azcore.centralus').database('Fa').HyperVHypervisorTable
   | where PreciseTimeStamp between (fn_faultTime - 10m .. fn_faultTime + 1m)
   | where NodeId == fn_nodeId
   | project PreciseTimeStamp, ProviderName, TaskName, Opcode, Message, Tid, EventId, Level
   | sort by PreciseTimeStamp asc
   ```
3. **Check Hawkeye RCA**: `cluster('hawkeyedataexplorer.westus2').database('HawkeyeLogs').HawkeyeRCAEvents`
4. **Check Hypervisor SEL events** (AH2021+): `GetHypervisorSELEventsForNode()`
5. **If dump is available**: Use `awdump.exe create live -hv` (the `-hv` flag is mandatory for hypervisor pages), load `hvexts.dll`, and use `dx @$cursession.Hvx.CreateHvView()` then `dx @$cursession.Hvx.SysLog`

### 10.2 VM Failure — Is the Hypervisor to Blame?

1. **Check VmHealthRawStateEtwTable** for heartbeat timeline
2. **Check WindowsEventsTable** for guest errors (triple faults, crashes)
3. **Check HyperVHypervisorTable** for hypervisor-level errors around the fault time (`Level <= 2`)
4. **Check HyperVWorkerTable** for VM lifecycle events
5. If hypervisor has errors at the fault time → hypervisor issue. Otherwise → guest or worker issue.

### 10.3 Live Migration Failure — Processor Feature Mismatch

1. **HyperVHypervisorTable** → `TaskName == "Vp config"` on both source and destination nodes
2. **HyperVVmConfigSnapshot** → VM's configured `ProcessorFeatureSet`
3. **HyperVWorkerTable** → EventId 18609 shows features VM sent to hypervisor
4. **Compare**: If VM's feature bit is set but destination node's feature is not → mismatch

### 10.4 Node Fault — Is Hyper-V to Blame?

1. **Hawkeye RCA** → Automated root cause analysis
2. **HyperVHypervisorTable** → Errors around fault time
3. **WindowsEventsTable** → Guest errors that might have cascaded
4. **HostOsVersion** → Was a recent host OS update applied?

---

## 11. When to Use This Table

### Use HyperVHypervisorTable When:
- Investigating **host bugchecks** with `HYPERVISOR_ERROR` crash buckets
- Checking **partition lifecycle** (creation/deletion) on a node
- Investigating **device assignment** events (attach/detach to partitions)
- Comparing **processor features** between nodes for live migration compatibility
- Investigating **hypervisor soft restart (HSR)** or **hypervisor hot restart (HHR)** during host updates
- Looking for **hypervisor loader failures** (`HvldrFailureLocation`)
- Checking **hypervisor version** and **hotpatch state**

### Do NOT Use This Table When:
- Looking for **guest OS events** → Use `UnderhillEventTable` (for Underhill/OpenHCL VMs) or `WindowsEventsTable` (for guest crashes, triple faults, boot events)
- Looking for **VM lifecycle events** (start, stop, save, restore) → Use `HyperVWorkerTable` or `HyperVVmmsTable`
- Looking for **VM configuration** → Use `HyperVVmConfigSnapshot` (AH2023+ only)
- Looking for **VM health/heartbeat** → Use `VmHealthRawStateEtwTable`
- Looking for **storage issues** → Use `HyperVStorageStackTable`
- Looking for **VPCI/device passthrough issues** → Use `HyperVVPciTable`
- Looking for **container metadata** (subscription, VM type) → Use `MycroftContainerSnapshot` in `AzureCP` database

---

## 12. Tips, Gotchas, and Known Issues

1. **Message JSON varies by TaskName/EventId** — Always use `parse_json(Message)` and inspect field names. Do not assume a fixed schema.
2. **Processor feature values are bitmasks** — They need to be compared bit-by-bit between source/destination nodes and the VM's configured features. Use `tohex()` for readable comparison.
3. **VM-PHU Self exception** — During VM-PHU Self updates, hypervisor logs are available in this table (unlike other Hyper-V tables which are ingested into the VM-PHU Trommel table). Hypervisor logs are the exception.
4. **ContainerId is not a column** — To filter by container, use `where Message has fn_containerId`. The ContainerId appears inside the JSON `Message` payload.
5. **VmId mapping** — VmId is a Hyper-V internal construct different from ContainerId. Map via `HyperVWorkerTable` with `TaskName == "VmNameToIdMapping"`.
6. **Log retention** — Hyper-V logs typically have ~60 day retention. If events are too old, they may have aged out.
7. **EventId 0 dominates** — ~70% of events have `EventId == 0`. For these diagnostics events, use `TaskName` as the primary discriminator.
8. **Sovereign Clouds** — Standard Kusto queries may not work for Government/FairFax/Mooncake environments. Refer to the Sovereign Cloud Kusto Queries TSG.
9. **Known benign filter for cross-table queries** — When querying `HyperVVmmsTable`, filter out WMI noise: `Message !contains "WHERE clause operator" and Message !contains "Provider could not handle query"`.
