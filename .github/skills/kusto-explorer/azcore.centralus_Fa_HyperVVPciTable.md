# HyperVVPciTable

**Type:** Table
**Cluster:** `https://azcore.centralus.kusto.windows.net`
**Database:** `Fa`
**Full Path:** `azcore.centralus.kusto.windows.net` → `Fa` → `HyperVVPciTable`

---

## 1. Description

**HyperVVPciTable** contains events and traces for VPCI (Virtual PCI) and the device assignment stack. The relevant data is stored in the JSON **Message** field. This table is the primary source for investigating device assignment issues including:

- **MANA** — networking adapters
- **NVMe Direct** — local storage devices
- **ASAP** — OS disk and remote data disks
- **SCSI** — ISO file devices
- **GPU** — GPU device assignment (GPU-P)

The table captures events from five ETW providers that together cover the full VPCI stack: the VPCI core driver (`vmvpci.dll`), the VPCI VSP (Virtualization Service Provider), the PCI physical device driver (PCIP), FlexIO, and the GPU-P virtual device driver.

**Data Volume:** ~38 billion rows/day (extremely high volume). The vast majority (~95%) are Level 4 (Informational) events. Error and warning events are ~2.4% and ~2.5% respectively.

**Cluster Shorthand:** `cluster('azcore.centralus').database('Fa')`

---

## 2. Schema

| Column | Type | Description |
|---|---|---|
| `TIMESTAMP` | datetime | Ingestion timestamp |
| `PreciseTimeStamp` | datetime | High-precision event timestamp (primary time filter) |
| `Environment` | string | Deployment environment (e.g., `PROD`) |
| `Region` | string | Azure region (e.g., `centralus`) |
| `DataCenter` | string | Data center code (e.g., `DSM12`) |
| `Cluster` | string | Cluster name within data center |
| `NodeIdentity` | string | Node IP address |
| `NodeId` | string | Host node identifier (GUID) — primary node filter |
| `DeviceId` | string | Device identifier (prefixed, e.g., `s:GUID`) |
| `Level` | long | Event severity: 1=Critical, 2=Error, 3=Warning, 4=Informational, 5=Verbose |
| `ProviderGuid` | string | ETW provider GUID |
| `ProviderName` | string | ETW provider name (see Provider breakdown below) |
| `EventId` | long | ETW event ID (almost always 0; 1520 for GPU-P events) |
| `Pid` | long | Process ID |
| `Tid` | long | Thread ID |
| `OpcodeName` | string | Operation code name |
| `KeywordName` | string | ETW keyword name |
| `TaskName` | string | Task name — key for filtering event categories |
| `ChannelName` | string | ETW channel name |
| `EventMessage` | string | Formatted event message (primarily used by GPU-P EventId=1520 events) |
| `ActivityId` | string | Activity correlation ID |
| `Task` | long | Numeric task identifier |
| `Opcode` | long | Numeric operation code |
| `RelatedActivityId` | string | Related activity correlation ID |
| `Message` | string | **JSON blob** — the primary data field containing event payload |
| `__AuthType__` | string | Authentication type |
| `__AuthIdentity__` | string | Authentication identity |
| `SourceNamespace` | string | Source namespace (e.g., `Fa`) |
| `SourceMoniker` | string | Source moniker (e.g., `FaDiagdm`) |
| `SourceVersion` | string | Source version |
| `AutopilotEnvironment` | string | Autopilot environment string |

---

## 3. Critical Column Guide — What to Query By

### Primary Filters (always include these)

| Column | Usage |
|---|---|
| `PreciseTimeStamp` | **Always** filter by time range first. Use `between (startTime .. endTime)` or `> ago(Xh)`. |
| `NodeId` | Filter by host node GUID for node-specific investigations. |

### Key Discriminators

| Column | Usage |
|---|---|
| `Level` | Filter by severity. Use `Level <= 2` for errors, `Level == 3` for warnings, `Level <= 4` for all non-verbose. |
| `TaskName` | Primary event category filter. See TaskName reference below. |
| `ProviderName` | Filter by component. Five providers exist (see below). |
| `Message` | JSON blob — use `has` for text search, `parse_json(Message)` for structured access. Contains `instanceGuid`, `emulatorId`, `emulatorType`, HRESULT codes, source file info, etc. |
| `EventMessage` | Used primarily for GPU-P events (EventId=1520). Contains formatted error strings with VM GUIDs and call stacks. |
| `DeviceId` | Filter by specific device identifier. |

### ProviderName Breakdown

| ProviderName | Events/Day | Component |
|---|---|---|
| `Microsoft.Windows.HyperV.VPCI` | ~20B | Core VPCI driver (`vmvpci.dll`) — device emulation, MMIO, config space |
| `Microsoft.Windows.HyperV.VPCIVSP` | ~12B | VPCI VSP — host-side service provider, protocol messages, handle counts |
| `Microsoft.Windows.HyperV.PCIP` | ~5B | PCI physical device driver — hardware allocation, power policy, register mitigation |
| `Microsoft.Windows.HyperV.FlexIo` | ~747M | FlexIO — flexible I/O subsystem |
| `Microsoft.Windows.HyperV.GpupVDev` | ~13M | GPU-P virtual device — GPU partitioning, UMED calls |

### Level Distribution (typical day)

| Level | Count | Percentage |
|---|---|---|
| 4 (Informational) | ~36.4B | 95.1% |
| 3 (Warning) | ~942M | 2.5% |
| 2 (Error) | ~920M | 2.4% |
| 5 (Verbose) | ~18M | <0.1% |

> **Note:** No Level 1 (Critical) events observed in typical daily data.

### Top TaskNames

| TaskName | Events/Day | Description |
|---|---|---|
| `VpciVspFileOp` | ~10.8B | VSP file operations |
| `VpciTrace` | ~9.8B | General VPCI trace events |
| `VpciProcessProtocolMessage` | ~6.4B | Protocol message processing between host/guest |
| `PcipTrace` | ~5.0B | PCI physical driver trace events |
| `VpciComMethod` | ~2.5B | COM method invocations |
| `FallbackError` | ~871M | Error fallback events (WIL error reporting) |
| `VpciVspTrace` | ~859M | VSP-specific trace events |
| `VpciVspHandleCounts` | ~750M | VSP handle count tracking |
| `EPciTrace` | ~542M | Emulated PCI trace (ATS, config space) |
| `EmulatorConfigurationString` | ~152M | Emulator configuration strings |
| `DeviceStateChange` | ~87M | Device lifecycle state transitions |
| `EmulatorRangeChangeBegin/End` | ~52M each | Emulator MMIO range changes |
| `EmulatorSetConfig` | ~33M | Emulator configuration updates |
| `EmulatorInit` | ~32M | Emulator initialization |
| `EmulatorTeardown` | ~32M | Emulator teardown |
| `AllocateHardware` | ~31M | Hardware resource allocation |
| `FreeHardware` | ~31M | Hardware resource deallocation |
| `EmulatorStart` | ~30M | Emulator start |
| `EmulatorStop` | ~30M | Emulator stop |

### Message JSON Key Fields

| JSON Field | Description |
|---|---|
| `instanceGuid` | The **Virtual System Identifier** (VSID) — the GUID assigned to the vdev on the host that corresponds to a VTL2-present device. This is the primary key for mapping Underhill error GUIDs to host-side devices. |
| `emulatorId` | GUID identifying the specific emulator/device type. Use this to map to the correct emulator type when `emulatorType` is not populated. |
| `emulatorType` | Human-readable device type string (e.g., `"ASAP"`, `"NVMeDirect"`, `"SCSI"`, `"MANA"`). |
| `wilResult_hresult` | HRESULT error code from WIL (Windows Implementation Library) error reporting. |
| `wilResult_fileName` | Source file where the error originated (e.g., `onecore\vm\dv\vpci\core2\virtualbus.cpp`). |
| `wilResult_message` | Detailed error message with call context. |
| `wilResult_callContext` | Call stack context string (e.g., `\Vpci::Core::VirtualBusMmioHandler::NotifyMmioRead`). |
| `wilResult_module` | Module name (e.g., `vmvpci.dll`). |
| `source` | Source file name (in PcipTrace / EPciTrace events). |
| `line` | Source line number. |
| `message` | Human-readable trace message (in PcipTrace / EPciTrace events). |

---

## 4. Key Event IDs Reference

| EventId | Provider | Description |
|---|---|---|
| 0 | All VPCI providers | Default event ID for all TraceLogging-based events (vast majority of events). Use `TaskName` and `Message` content to distinguish. |
| 1520 | `Microsoft.Windows.HyperV.GpupVDev` | GPU-P virtual device errors. Contains formatted `EventMessage` with VM GUID, source file, HRESULT, and call context (e.g., `GpupPostReset`, `GpupPowerOff`, `GpupReserveResources`). |

---

## 5. Common Message Patterns

### Error Events (Level 2) — FallbackError TaskName
WIL error reports from `vmvpci.dll` with structured JSON containing HRESULT codes, source locations, and call contexts:
```
{"wilResult_hresult":2147942421, "wilResult_fileName":"onecore\\vm\\dv\\vpci\\core2\\virtualbus.cpp",
 "wilResult_message":"... 80070015 The device is not ready. CallContext:[\\Vpci::Core::VirtualBusMmioHandler::NotifyMmioRead]",
 "wilResult_module":"vmvpci.dll"}
```

### Warning Events (Level 3) — PcipTrace TaskName
PCI physical device driver warnings, often related to power policy or register mitigation:
```
{"source":"SetPowerPolicy","line":1633,"message":"WdfDeviceAssignSxWakeSettings failed (possibly buggy BIOS DeviceWake=D0). Status: 0xC00002D3"}
{"source":"InitializeMitigationMaps","line":237,"message":"WdfRegistryQueryValue failed. Status: 0xC0000034"}
{"source":"ReadMitigatedRegister","line":410,"message":"Rd 0000000000000BCC: 0b  (real value was 10)"}
```

### Informational Events (Level 4) — EPciTrace TaskName
Emulated PCI configuration space reads, ATS control register operations:
```
{"source":"EPciExtCapAts.h","line":46,"message":"09:01:00:00(000) [ATS] Read of ATS control register, enable: 1"}
```

### GPU-P Events (EventId 1520) — EventMessage Field
Formatted error strings from GPU partitioning operations:
```
[Virtual machine B41513D6-C6A9-4D47-B80F-D760DF190949] onecore\vm\dv\gpup\dll\gpupvdev.cpp(1515)\gpupvdev.dll: Exception(1) tid(52f0) 80004005 Unspecified error
    CallContext:[\GpupPostReset\GpupCreateUMED\GpupUMEDCall]
```

---

## 6. Sample Queries

### Query 1: Map Virtual System Identifier (VSID) to Device Type

When an Underhill error message contains a GUID (the "Virtual System Identifier"), use this query to determine which device type it maps to. This is critical for routing storage-related Underhill failures to the correct team.

```kusto
let fn_nodeId = '<node-id-guid>';
let fn_startTime = datetime(2024-04-02T18:00:44Z);
let fn_endTime = datetime(2024-04-02T18:16:44Z);
let fn_vsid = "<virtual-system-identifier-guid>";
cluster('azcore.centralus').database('Fa').HyperVVPciTable
| where NodeId == fn_nodeId and PreciseTimeStamp between (fn_startTime .. fn_endTime)
| extend msg = parse_json(Message)
| where tostring(msg.instanceGuid) == fn_vsid
| distinct fn_vsid, tostring(msg.emulatorId), tostring(msg.emulatorType)
```

**Device Type Interpretation:**

| emulatorType | Device Category | Description | Escalation Queue |
|---|---|---|---|
| `ASAP` | Storage | OS disk and remote data disks | Host Storage Acceleration / Triage |
| `NVMe Direct` | Storage | Local storage (NVMe Direct devices) | zHYP SME DAS (HYP SME use only) |
| `SCSI` | Storage | ISO file | zHYP SME SVP (HYP SME use only) |
| `MANA` | Networking | Network adapter | Host Networking / Triage |
| `GPU` | Compute | GPU device assignment | (varies) |

> **Note:** If `emulatorType` is not populated, use `emulatorId` to map to the correct emulator type. If you cannot identify the device, reach out to **zHYP SME SVP (HYP SME use only)** oncall.

### Query 2: Find Error Events for a Node

```kusto
HyperVVPciTable
| where PreciseTimeStamp between (datetime(2025-01-01) .. datetime(2025-01-02))
| where NodeId == '<node-id>'
| where Level <= 2
| extend msg = parse_json(Message)
| project PreciseTimeStamp, Level, TaskName, ProviderName,
    hresult = tostring(msg.wilResult_hresult),
    source = coalesce(tostring(msg.wilResult_fileName), tostring(msg.source)),
    detail = coalesce(tostring(msg.wilResult_message), tostring(msg.message))
| order by PreciseTimeStamp asc
```

### Query 3: Device Lifecycle Events for a Container

```kusto
HyperVVPciTable
| where PreciseTimeStamp between (datetime(2025-01-01) .. datetime(2025-01-02))
| where NodeId == '<node-id>'
| where Message has '<container-id>'
| where TaskName in ('EmulatorInit', 'EmulatorStart', 'EmulatorStop', 'EmulatorTeardown',
    'DeviceStateChange', 'AllocateHardware', 'FreeHardware')
| extend msg = parse_json(Message)
| project PreciseTimeStamp, TaskName, Level,
    emulatorType = tostring(msg.emulatorType),
    instanceGuid = tostring(msg.instanceGuid)
| order by PreciseTimeStamp asc
```

### Query 4: GPU-P Error Events

```kusto
HyperVVPciTable
| where PreciseTimeStamp > ago(1d)
| where EventId == 1520
| where ProviderName == "Microsoft.Windows.HyperV.GpupVDev"
| project PreciseTimeStamp, NodeId, EventMessage
| order by PreciseTimeStamp desc
| take 100
```

### Query 5: VPCI Protocol Message Analysis

```kusto
HyperVVPciTable
| where PreciseTimeStamp between (datetime(2025-01-01) .. datetime(2025-01-02))
| where NodeId == '<node-id>'
| where TaskName == 'VpciProcessProtocolMessage'
| where Message has '<container-id>'
| extend msg = parse_json(Message)
| project PreciseTimeStamp, Level, msg
| order by PreciseTimeStamp asc
```

---

## 7. Cross-Table Query Patterns

### Combined Underhill + HyperV Tables Timeline (Union Query)

View a combined timeline of event logging from Underhill, VMMS, Worker, Hypervisor, and VPCI tables. This gives a unified view of all Hyper-V subsystem events for a container.

```kusto
let fn_nodeId = '<node-id>';
let fn_containerId = '<container-id>';
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

### Cross-References to Other Tables

| Table | Cluster / Database | Relationship |
|---|---|---|
| **UnderhillEventTable** | `cluster('wdgeventstore.kusto.windows.net').database('AzureHostOs')` | Underhill (VTL2) events. Error messages here often contain GUIDs that map to VPCI device entries via `instanceGuid`. |
| **HyperVWorkerTable** | `cluster('azcore.centralus').database('Fa')` | Worker process (vmwp.exe) events. Manages runtime of VMs. Union'd with VPCI table in combined queries. |
| **HyperVVmmsTable** | `cluster('azcore.centralus').database('Fa')` | VM Management Service events. Union'd with VPCI table in combined queries. |
| **HyperVHypervisorTable** | `cluster('azcore.centralus').database('Fa')` | Hypervisor events. Union'd with VPCI table in combined queries. |
| **HyperVStorageStackTable** | `cluster('azcore.centralus').database('Fa')` | Storage virtualization events. For NVMe Direct errors, check this for detailed NVMe Direct driver-level errors (ProviderName `Microsoft.Windows.HyperV.Storage.NvmeDirect*`). |
| **HyperVVmConfigSnapshot** | `cluster('azcore.centralus').database('Fa')` | VM configuration snapshots. Used to determine if a VM is an Underhill VM (`IsUnderhill` field). |
| **MycroftContainerSnapshot** | `cluster('azcore.centralus').database('AzureCP')` | Container metadata including VM type, Trusted VM status, subscription IDs. |
| **MycroftContainerHealthSnapshot** | `cluster('azcore.centralus').database('AzureCP')` | Container health/state as seen by control layers above Hyper-V. |
| **HyperVTdprEvents** | `cluster('azcore.centralus').database('Fa')` | TDPR-style timeline/graph events based on HyperV.Regions.xml. |

---

## 8. Investigation Playbook

### Pattern 1: Underhill Storage Device Failure Triage

**When:** Underhill error message contains `nvme` or `StorageCannotOpenVtl2Device`.

**Steps:**
1. Extract the GUID from the Underhill error message — this is the **Virtual System Identifier** (VSID).
2. Run **Query 1** (Map VSID to Device Type) using that GUID as `fn_vsid`.
3. Check `msg.emulatorType` to identify the device category.
4. Route to the appropriate team based on device type (see Query 1 table).

### Pattern 2: Underhill MANA (Networking) Failure Triage

**When:** Underhill error message contains `mana`.

**Steps:**
- If message contains `"failed to start mana device"` → Reach out to **Host Networking / Triage**.
- Otherwise → Reach out to **zHYP SME LOW (HYP SME use only)**.

### Pattern 3: Underhill VMGS (NVRAM) Failure Triage

**When:** Underhill error message contains `vmgs`.

**Steps:**
- This relates to NVRAM variables. Reach out to **zHYP SME MVM (HYP SME use only)**.

### Pattern 4: Combined Timeline Analysis

**When:** You need a complete picture of what happened across all Hyper-V subsystems.

**Steps:**
1. Run the Combined Union Query (Section 7) to get a unified timeline.
2. Filter by `Table == "vpci"` to isolate VPCI-specific events.
3. Correlate VPCI events with other subsystem events using `PreciseTimeStamp`.

### Pattern 5: VPCI StopDestroy Timeout (Known Issue)

There is a known issue in VPCI that can cause StopDestroy timeouts. See the [Stop Container Failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/stop-container-failure) for queries to determine whether this issue applies.

### Pattern 6: NVMe Direct Device Assignment Errors

For NVMe Direct-specific errors surfacing through device assignment, also investigate through the **HyperVStorageStackTable** using the NVMe Direct provider names (`Microsoft.Windows.HyperV.Storage.NvmeDirect*`).

### Escalation Paths

| Scenario | IcM Queue / Contact |
|---|---|
| ASAP device (OS disk, remote data disks) | Host Storage Acceleration / Triage |
| NVMe Direct devices (local storage) | zHYP SME DAS (HYP SME use only) |
| SCSI device (ISO file) | zHYP SME SVP (HYP SME use only) |
| MANA - failed to start mana device | Host Networking / Triage |
| MANA - other | zHYP SME LOW (HYP SME use only) |
| VMGS / NVRAM variables | zHYP SME MVM (HYP SME use only) |
| Unable to identify device | zHYP SME SVP (HYP SME use only) oncall |
| General device assignment | Contact: `vpcidev` |
| General NVMe Direct | Contact: `nvmedirect` |
| General Hyper-V virtualization | RDOS/Azure Host OS SME - Virtualization (Hyper-V) |

---

## 9. When to Use This Table

**Use HyperVVPciTable when:**
- Investigating device assignment failures (MANA, NVMe Direct, ASAP, SCSI, GPU)
- Mapping Underhill error GUIDs (Virtual System Identifiers) to device types
- Debugging VPCI protocol message failures between host and guest
- Investigating device lifecycle issues (init, start, stop, teardown)
- Analyzing GPU-P (GPU partitioning) errors
- Correlating host-side VPCI events with Underhill VTL2 errors
- Investigating StopDestroy timeouts related to VPCI
- Building combined timelines across Hyper-V subsystems

**Do NOT use this table for:**
- Storage stack details below VPCI level → use **HyperVStorageStackTable**
- VM management operations (create, delete, configure) → use **HyperVVmmsTable**
- Worker process events → use **HyperVWorkerTable**
- Hypervisor-level events → use **HyperVHypervisorTable**
- Underhill/VTL2-side logs → use **UnderhillEventTable**

---

## Source Pages

1. [Hyper-V Kusto Queries](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/hyperv-kusto-queries)
2. [Underhill TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-tsg)
3. [Underhill Kusto Queries FAQ](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-kusto-queries-faq)
4. [Stop Container Failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/stop-container-failure)
5. [NVMe Direct Errors TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/deviceassignment/nvme-direct-errors)

**Maintainer:** Contact `hypsme` | IcM queue: RDOS/Azure Host OS SME - Virtualization (Hyper-V)
