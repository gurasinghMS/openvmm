# HyperVVPciTable Investigation Guide

> Distilled from RDOS Livesite EngHub documentation.
> Covers: HyperVVPciTable in azcore.centralus / Fa

---

## Table Overview

**HyperVVPciTable** contains events and traces for VPCI and the device assignment stack. The relevant data is stored in the JSON **Message** field. This table is the primary source for investigating device assignment issues including MANA (networking), NVMe Direct (local storage), ASAP (OS disk / remote data disks), SCSI (ISO), and GPU devices.

### Cluster / Database Location

```
cluster('azcore.centralus.kusto.windows.net').database('Fa')
```

Shorthand: `cluster('azcore.centralus').database('Fa')`

Execute links:
- Web: https://azcore.centralus.kusto.windows.net/Fa
- Desktop / Web (Lens) / Desktop (SAW) also available

---

## Key Fields

| Field | Description |
|---|---|
| `PreciseTimeStamp` | High-precision event timestamp |
| `NodeId` | Host node identifier (GUID) |
| `Message` | **JSON blob** containing event payload — the primary data field |
| `EventMessage` | Formatted event message string |
| `Level` | Event severity level (1=Critical, 2=Error, 3=Warning, 4=Informational, 5=Verbose) |
| `TaskName` | Task name associated with the event |
| `Opcode` | Operation code for the event |
| `ActivityId` | Activity correlation ID |
| `RelatedActivityId` | Related activity correlation ID |

### Message JSON Key Fields

| JSON Field | Description |
|---|---|
| `instanceGuid` | The **Virtual System Identifier** (VSID) — the GUID assigned to the vdev on the host that corresponds to a VTL2-present device. This is the primary key for mapping Underhill error GUIDs to host-side devices. |
| `emulatorId` | GUID identifying the specific emulator/device type. Use this to map to the correct emulator type when `emulatorType` is not populated. |
| `emulatorType` | Human-readable device type string (e.g., `"ASAP"`, `"NVMeDirect"`, `"SCSI"`, `"MANA"`). |

---

## All Kusto Queries

### Query 1: Map Virtual System Identifier to Device Type

**Source:** [Underhill TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-tsg)

**Purpose:** When an Underhill error message contains a GUID (the "Virtual System Identifier"), use this query to determine which device type it maps to. This is critical for routing storage-related Underhill failures to the correct team.

```kusto
let fn_nodeId = 'd71bdb10-080b-705a-ed75-568665161908';
let fn_startTime = datetime(2024-04-02T18:00:44Z);
let fn_endTime = datetime(2024-04-02T18:16:44Z);
let fn_vsid = "cc49b599-d9b8-46c9-aaf4-1240c59a6a9b";
cluster('azcore.centralus').database('Fa').HyperVVPciTable
| where NodeId == fn_nodeId and PreciseTimeStamp between (fn_startTime .. fn_endTime)
| extend msg = parse_json(Message)
| where tostring(msg.instanceGuid) == fn_vsid
| distinct fn_vsid, tostring(msg.emulatorId), tostring(msg.emulatorType)
```

**Example Output:**

| fn_vsid | msg.emulatorId | msg.emulatorType |
|---|---|---|
| cc49b599-d9b8-46c9-aaf4-1240c59a6a9b | 7ee2e239-4a72-4367-97db-d079d9a96d59 | ASAP |

**Device Type Interpretation:**

| emulatorType | Device Category | Description | Escalation Queue |
|---|---|---|---|
| `ASAP` | Storage | OS disk and remote data disks | Host Storage Acceleration / Triage |
| NVMe Direct | Storage | Local storage (NVMe Direct devices) | zHYP SME DAS (HYP SME use only) |
| `SCSI` | Storage | ISO file | zHYP SME SVP (HYP SME use only) |
| `MANA` | Networking | Network adapter | Host Networking / Triage |
| GPU | Compute | GPU device assignment | (varies) |

**Notes:**
- If `emulatorType` is not populated, use `emulatorId` to map to the correct emulator type.
- If you have problems identifying the device, reach out to **zHYP SME SVP (HYP SME use only)** oncall.

---

### Query 2: Combined Underhill + HyperV Tables Timeline (Union Query)

**Source:** [Underhill Kusto Queries FAQ](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-kusto-queries-faq)

**Purpose:** View a combined timeline of event logging from Underhill Event Table, HyperV VMMS Table, HyperV VM Worker Process Table, HyperV Hypervisor Table, and HyperV VPCI Table. This gives a unified view of all Hyper-V subsystem events for a container.

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
    //| where Level <= 4 // Filter out GuestEmulationDevice::HandleRequest logs
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

**Notes:**
- The `vpci` let-binding filters HyperVVPciTable by NodeId, ContainerId (via `Message has`), time range, and Level <= 4.
- The Message JSON is parsed, cleaned of noise fields, and flattened.
- The `Table` column in the output indicates which source table each row came from: `"uh"`, `"vmms"`, `"vmwp"`, `"vmhv"`, or `"vpci"`.
- The VMMS sub-query filters out annoying spam from LIKE queries (`"WHERE clause operator"`, `"Provider could not handle query"`).

---

## Common Investigation Patterns

### Pattern 1: Underhill Storage Device Failure Triage

**When:** Underhill error message contains `nvme` or `StorageCannotOpenVtl2Device`.

**Steps:**
1. Extract the GUID from the Underhill error message — this is the **Virtual System Identifier** (VSID).
2. Run **Query 1** (Map Virtual System Identifier to Device Type) using that GUID as `fn_vsid`.
3. Check `msg.emulatorType` to identify the device category.
4. Route to appropriate team based on device type (see table in Query 1).

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
1. Run **Query 2** (Combined Union Query) to get a unified timeline.
2. Filter by `Table == "vpci"` to isolate VPCI-specific events.
3. Correlate VPCI events with other subsystem events using `PreciseTimeStamp`.

### Pattern 5: VPCI StopDestroy Timeout (Known Issue)

**Source:** [Stop Container Failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/stop-container-failure)

There is a known issue in VPCI that can cause StopDestroy timeouts. See the known issues page for queries to run to determine whether this issue applies.

### Pattern 6: NVMe Direct Device Assignment Errors

For NVMe Direct-specific errors surfacing through device assignment, also investigate through the **HyperVStorageStackTable** (see [HyperVStorageStackTable Investigation Guide](./HyperVStorageStackTableInvestigation.md)) using the NVMe Direct provider names.

---

## Cross-References to Other Tables

| Table | Cluster / Database | Relationship |
|---|---|---|
| **UnderhillEventTable** | `cluster('wdgeventstore.kusto.windows.net').database('AzureHostOs')` | Underhill (VTL2) events. Error messages here often contain GUIDs that map to VPCI device entries via `instanceGuid`. |
| **HyperVWorkerTable** | `cluster('azcore.centralus').database('Fa')` | Worker process (vmwp.exe) events. Manages runtime of VMs. Union'd with VPCI table in combined queries. |
| **HyperVVmmsTable** | `cluster('azcore.centralus').database('Fa')` | VM Management Service events. Union'd with VPCI table in combined queries. |
| **HyperVHypervisorTable** | `cluster('azcore.centralus').database('Fa')` | Hypervisor events. Union'd with VPCI table in combined queries. |
| **HyperVStorageStackTable** | `cluster('azcore.centralus').database('Fa')` | Storage virtualization events. For NVMe Direct errors, check this table for detailed NVMe Direct driver-level errors (ProviderName `Microsoft.Windows.HyperV.Storage.NvmeDirect*`). |
| **HyperVVmConfigSnapshot** | `cluster('azcore.centralus').database('Fa')` | VM configuration snapshots. Used to determine if a VM is an Underhill VM (`IsUnderhill` field). |
| **MycroftContainerSnapshot** | `cluster('azcore.centralus').database('AzureCP')` | Container metadata including VM type, Trusted VM status, subscription IDs. |
| **MycroftContainerHealthSnapshot** | `cluster('azcore.centralus').database('AzureCP')` | Container health/state as seen by control layers above Hyper-V. |
| **HyperVTdprEvents** | `cluster('azcore.centralus').database('Fa')` | TDPR-style timeline/graph events based on HyperV.Regions.xml. |

---

## Escalation Paths

| Scenario | IcM Queue / Contact |
|---|---|
| ASAP device (OS disk, remote data disks) | Host Storage Acceleration / Triage |
| NVMe Direct devices (local storage) | zHYP SME DAS (HYP SME use only) |
| SCSI device (ISO file) | zHYP SME SVP (HYP SME use only) |
| MANA - failed to start mana device | Host Networking / Triage |
| MANA - other | zHYP SME LOW (HYP SME use only) |
| VMGS / NVRAM variables | zHYP SME MVM (HYP SME use only) |
| Unable to identify device or having problems | zHYP SME SVP (HYP SME use only) oncall |
| General device assignment | Contact: `vpcidev` |
| General NVMe Direct | Contact: `nvmedirect` |
| General Hyper-V virtualization | RDOS/Azure Host OS SME - Virtualization (Hyper-V) |

---

## Source Pages

1. [Hyper-V Kusto Queries](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/hyperv-kusto-queries)
2. [Underhill TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-tsg)
3. [Underhill Kusto Queries FAQ](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-kusto-queries-faq)
4. [Stop Container Failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/stop-container-failure)
5. [NVMe Direct Errors TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/deviceassignment/nvme-direct-errors)

**Maintainer:** Contact `hypsme` | IcM queue: RDOS/Azure Host OS SME - Virtualization (Hyper-V)
