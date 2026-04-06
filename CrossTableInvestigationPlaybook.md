# Cross-Table Investigation Playbook

> How to investigate Azure VM issues across Kusto tables: boot process, table correlation, investigation flows, and incident triage.
> Distilled from RDOS Livesite EngHub documentation.

---

## Table of Contents

1. [Kusto Cluster Map](#1-kusto-cluster-map)
2. [Complete Table Catalog](#2-complete-table-catalog)
3. [Key Identifiers and Join Fields](#3-key-identifiers-and-join-fields)
4. [VM Boot Process — Table-by-Table](#4-vm-boot-process--table-by-table)
5. [Cross-Table Correlation Techniques](#5-cross-table-correlation-techniques)
6. [Investigation Flows](#6-investigation-flows)
   - [6.1 VM Won't Start (Container Start Failure)](#61-vm-wont-start-container-start-failure)
   - [6.2 VM Unexpected Reboot](#62-vm-unexpected-reboot)
   - [6.3 VM Crash / BSOD / Triple Fault](#63-vm-crash--bsod--triple-fault)
   - [6.4 VM Slow / Performance Issues](#64-vm-slow--performance-issues)
   - [6.5 Underhill (VTL2) Crash](#65-underhill-vtl2-crash)
   - [6.6 Live Migration Failure](#66-live-migration-failure)
   - [6.7 Storage Issues](#67-storage-issues)
   - [6.8 Networking Issues](#68-networking-issues)
   - [6.9 Servicing / Deployment Failures](#69-servicing--deployment-failures)
   - [6.10 Node Reboot / Host Crash](#610-node-reboot--host-crash)
   - [6.11 VM Creation Failure (SLA Drop)](#611-vm-creation-failure-sla-drop)
7. [Incident Triage — Team Ownership](#7-incident-triage--team-ownership)
8. [Tools and Dashboards](#8-tools-and-dashboards)
9. [Quick Reference: Common Queries](#9-quick-reference-common-queries)

---

## 1. Kusto Cluster Map

All RDOS investigations span multiple Kusto clusters. Here is the canonical map:

| Cluster | Database(s) | What Lives Here | Access SG |
|---------|-------------|-----------------|-----------|
| `azcore.centralus.kusto.windows.net` | **Fa** | All core HyperV tables (Worker, VMMS, Hypervisor, VPCI, StorageStack, VmConfig, VmHealth, OsFileVersion, WindowsEvents) | AzDeployer Kusto User SG |
| `azcore.centralus.kusto.windows.net` | **Fc** | `LiveMigrationSessionCompleteLog` | AzDeployer Kusto User SG |
| `azcore.centralus.kusto.windows.net` | **AzureCP** | `MycroftContainerHealthSnapshot` (container lifecycle states) | AzDeployer Kusto User SG |
| `azcore.centralus.kusto.windows.net` | **SharedWorkspace** | Shared functions: `AgentOperations()`, `HyperVContainerStarted()` | AzDeployer Kusto User SG |
| `wdgeventstore.kusto.windows.net` | **AzureHostOs** | `UnderhillEventTable` (VTL2/HCL events via Microsoft.Windows.HyperV.Hcl provider) | AzureHostOS Kusto Database Users |
| `wdgeventstore.kusto.windows.net` | **HostOSDeploy** | `AnyHostUpdateOnNode()` function, servicing/deployment data | AzureHostOS Kusto Database Users |
| `wdgeventstore.kusto.windows.net` | **CCA** | Servicing data, CBS deployment logs | AzureHostOS Kusto Database Users |
| `gandalf` (alias for `azurecm.kusto.windows.net`) | **AzureCM** | `TMMgmtNodeEventsEtwTable`, `LogContainerSnapshot` (node management, container snapshots) | — |
| `azdeployer.kusto.windows.net` | **AzDeployerKusto** | `ServiceVersionSwitch`, `OMWorkerRepairGenerator` (pilotfish deployment tracking) | AzDeployer Kusto User SG |
| `hawkeyedataexplorer.centralus.kusto.windows.net` | **HawkeyeLogs** | `HawkeyeRCAEvents`, `GetLatestFaultedNodes()` (automated RCA, fault triage) | — |
| `baseplatform.westus.kusto.windows.net` | **vmphu** | VM-PHU (Preserve Host Update) tables | VMPHU Kusto Viewer SG |
| `hostosdata.centralus.kusto.windows.net` | **HostOsData** | `OverlakeClusterVersions`, host OS data for RA tool | HostOsData Kusto Viewers SG |
| `hostosdata.centralus.kusto.windows.net` | **NFP** | Crash data, node fault processing | HostOsData Kusto Viewers SG |
| `vmainsight.kusto.windows.net` | **Air** | `LiveMigrationFailureEvents`, `AirLiveMigrationEvents` (detailed LM diagnostics) | — |
| `icmcluster.kusto.windows.net` | — | ICM incident data | IcM-Kusto-Access |
| `xstore.kusto.windows.net` | — | Azure Storage / XStore telemetry (Host Analyzer) | XLivesiteKustoAccess |
| `netperf.kusto.windows.net` | **NetPerfKustoDB** | MANA networking performance data | — |
| `storageclient.eastus.kusto.windows.net` | **Fa** | ASAP NVMe / storage client telemetry | — |
| `gandalffollower.centralus.kusto.windows.net` | — | Gandalf follower data | AlbusViewer SG |
| `sparkle.eastus.kusto.windows.net` | — | Sparkle diagnostics | SparkleUsers SG |

> **Tip:** For sovereign clouds (Fairfax/Mooncake), cluster URLs differ. See the Sovereign Cloud Kusto Queries TSG.

---

## 2. Complete Table Catalog

### 2.1 Core HyperV Tables (azcore.centralus / Fa)

| Table | What It Contains | Key Fields | Retention |
|-------|-----------------|------------|-----------|
| `HyperVWorkerTable` | VM worker process (vmwp.exe) events — VM start/stop, guest reset, device operations | `NodeId`, `ContainerId` (via `Message`), `EventId`, `PreciseTimeStamp` | ~30 days |
| `HyperVVmmsTable` | VMMS (Virtual Machine Management Service) events — VM creation, configuration, VHD operations | `NodeId`, `ContainerId` (via `EventMessage`), `Level`, `PreciseTimeStamp` | ~30 days |
| `HyperVHypervisorTable` | Hypervisor-level events — partition management, VP scheduling, bugchecks | `NodeId`, `ContainerId` (via `Message`), `TaskName`, `PreciseTimeStamp` | ~30 days |
| `HyperVVPciTable` | Virtual PCI events — device assignment, VPCI operations | `NodeId`, `ContainerId` (via `Message`), `PreciseTimeStamp` | ~30 days |
| `HyperVStorageStackTable` | Storage stack events — VHD operations, disk errors, VHDMP traces | `NodeId`, `Level`, `EventId`, `Message`, `PreciseTimeStamp` | ~30 days |
| `HyperVVmConfigSnapshot` | Periodic VM configuration snapshots — Underhill config, memory, VTL2 state | `NodeId`, `ContainerId`, `SummaryType`, `SummaryJson`, `IsUnderhill` | ~30 days |
| `VmHealthRawStateEtwTable` | VM health heartbeat — power state, IC heartbeat, VSC state | `ContainerId`, `NodeId`, `VmHyperVIcHeartbeat`, `VmPowerState`, `PreciseTimeStamp` | ~30 days |
| `OsFileVersionTable` | File versions on nodes — used to determine Underhill version, driver versions | `NodeId`, `FileName`, `FileVersion`, `PreciseTimeStamp` | ~30 days |
| `WindowsEventsTable` | General Windows event log entries from host | `NodeId`, `ContainerId`, `PreciseTimeStamp` | ~30 days |

### 2.2 Underhill / VTL2 Table (wdgeventstore / AzureHostOs)

| Table | What It Contains | Key Fields | Retention |
|-------|-----------------|------------|-----------|
| `UnderhillEventTable` | All Underhill (VTL2 Linux) events via Microsoft.Windows.HyperV.Hcl provider — boot, device, crash, diagnostics | `NodeId`, `VmName` (= ContainerId), `VmId`, `Level`, `TaskName`, `Message`, `TIMESTAMP` | ~30 days |

### 2.3 Container & Agent Tables

| Table | Cluster / Database | What It Contains | Key Fields |
|-------|--------------------|-----------------|------------|
| `MycroftContainerHealthSnapshot` | azcore.centralus / AzureCP | Container lifecycle state (Started, Stopped, etc.) | `ContainerId`, `NodeId`, `ContainerState`, `LifecycleState`, `PreciseTimeStamp` |
| `LogContainerSnapshot` | gandalf / AzureCM | Container deployment snapshots — subscription, role instance, tenant | `nodeId`, `containerId`, `virtualMachineUniqueId`, `subscriptionId`, `Tenant` |
| `VmServiceContainerOperations` | azcore.centralus / Fa | Container operations including Underhill flags | `NodeId`, `ContainerId`, `Cluster`, `IsUnderhillLocalEnabled` |

### 2.4 Node Management & Fault Tables

| Table | Cluster / Database | What It Contains | Key Fields |
|-------|--------------------|-----------------|------------|
| `TMMgmtNodeEventsEtwTable` | gandalf / AzureCM | Node-level management events — HE updates, reboots, state changes | `NodeId`, `Message`, `PreciseTimeStamp` |
| `HawkeyeRCAEvents` | hawkeyedataexplorer / HawkeyeLogs | Automated RCA for node/container faults | `ResourceId` (NodeId or ContainerId), `Scenario`, `RCALevel1`, `RCALevel2`, `EscalateToTeam` |
| `GetLatestFaultedNodes()` | hawkeyedataexplorer / HawkeyeLogs | Function: faulted nodes in a tenant | `NodeId`, `FaultReason`, `faultInfo` |

### 2.5 Live Migration Tables

| Table | Cluster / Database | What It Contains | Key Fields |
|-------|--------------------|-----------------|------------|
| `LiveMigrationSessionCompleteLog` | azcore.centralus / Fc | LM session outcomes — source/dest nodes and containers | `sessionId`, `sourceNodeId`, `sourceContainerId`, `destinationNodeId`, `destinationContainerId`, `vmUniqueId` |
| `LiveMigrationFailureEvents` | vmainsight / Air | LM failure diagnostics — RCA levels, session IDs | `NodeId`, `ObjectId`, `Diagnostics` (JSON with SessionId, HyperVVMId, destination info) |
| `AirLiveMigrationEvents` | vmainsight / Air | Detailed LM timing — brownout, blackout, port programming | `SessionId`, node and timing fields |

### 2.6 Deployment & Servicing Tables

| Table | Cluster / Database | What It Contains | Key Fields |
|-------|--------------------|-----------------|------------|
| `ServiceVersionSwitch` | azdeployer / AzDeployerKusto | Pilotfish package version changes on nodes | `NodeId`, `ServiceName`, `CurrentVersion`, `NewVersion`, `PreciseTimeStamp` |
| `OMWorkerRepairGenerator` | azdeployer / AzDeployerKusto | Repair/update operations triggered by OaaS | `azureNodeId`, `virtualEnvironment`, `assignedVersion`, `PreciseTimeStamp` |
| `AnyHostUpdateOnNode()` | wdgeventstore / HostOSDeploy | Function: any host OS update on a node during a time window | Inputs: `StartTime`, `EndTime`, `nodeList` |
| `OverlakeClusterVersions` | hostosdata.centralus / HostOsData | Cluster version info — ARM, generation, region | `Cluster`, `isARM`, `MajorGen`, `Region` |

---

## 3. Key Identifiers and Join Fields

Understanding the identifier hierarchy is critical for cross-table correlation:

| Identifier | Description | Scope | Where Used |
|-----------|-------------|-------|------------|
| **NodeId** | GUID for a physical host node | Global, stable | Every table |
| **ContainerId** | GUID for a VM incarnation on a node (= VmName in Hyper-V) | Per-node, temporal — changes on redeploy/migration | Most HyperV tables, agent tables |
| **VmId** | Hyper-V internal VM GUID | Per-node, per-lifetime | HyperVWorkerTable, HyperVVmmsTable |
| **VmUniqueId** | Azure resource-level VM ID (persists across migrations) | Global, stable | LogContainerSnapshot, LM tables |
| **Tenant** | Cluster name (e.g., "CDM03PrdApp04") | Stable | LogContainerSnapshot, various |
| **PreciseTimeStamp / TIMESTAMP** | Event timestamp | Universal | Every table |
| **sessionId** | Live migration session ID | Per-migration | LM tables |
| **ActivityId / RelatedActivityId** | Correlation IDs for operation chains | Per-operation | HyperV tables |

### ID Mapping Queries

**ContainerId → VmId:**
```kql
// Map ContainerId to Hyper-V VmId
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
cluster('azcore.centralus').database('Fa').HyperVWorkerTable
| where NodeId in (fn_nodeId)
| where ProviderName == "Microsoft.Windows.HyperV.Worker"
| where TaskName == "VmNameToIdMapping"
| project j = parse_json(Message)
| summarize by VmId = tostring(j.VmId), VmName = tostring(j.VmName)
| where VmName == fn_containerId
```

**Computer Name → NodeId + ContainerId:**
```kql
let fn_subscriptionId = '<SubscriptionId>';
let fn_roleInstanceName = "<ComputerName>";
cluster("AzureCM").database("AzureCM").LogContainerSnapshot
| where TIMESTAMP >= ago(7d)
| where subscriptionId == fn_subscriptionId
| where roleInstanceName has fn_roleInstanceName
| distinct nodeId, containerId, virtualMachineUniqueId, subscriptionId,
           roleInstanceName, Tenant, tipNodeSessionId
```

**Check if VM is Underhill:**
```kql
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
let fn_faultTime = datetime(<FaultTime>);
let fn_startTime = fn_faultTime - 1d;
let fn_endTime = fn_faultTime + 1h;
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot
| where NodeId == fn_nodeId and ContainerId == fn_containerId
    and PreciseTimeStamp between(fn_startTime .. fn_endTime)
| where SummaryType == "Configuration"
| extend IsUnderhillFromJson = parse_json(SummaryJson).Settings.hcl.IsUnderhill
| project PreciseTimeStamp,
    IsUnderhill = iff(isnotempty(IsUnderhill), IsUnderhill, IsUnderhillFromJson)
| order by PreciseTimeStamp desc
| take 1
```

---

## 4. VM Boot Process — Table-by-Table

When a VM is created and started on an Azure host, the following chain of events occurs. Each stage emits telemetry to specific Kusto tables:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  STAGE 1: Host Boot                                                         │
│  PXE → UEFI → Windows kernel → Hyper-V role enabled                        │
│  Table: TMMgmtNodeEventsEtwTable (gandalf/AzureCM)                         │
│  Check: Node event history, DoHostingEnvironmentUpdate messages             │
├──────────────────────────────────────────────────────────────────────────────┤
│  STAGE 2: Hypervisor Starts                                                 │
│  Hypervisor loads → partitions created → root VP scheduling begins          │
│  Table: HyperVHypervisorTable (azcore/Fa)                                  │
│  Check: Partition creation events, hypervisor bugchecks                     │
├──────────────────────────────────────────────────────────────────────────────┤
│  STAGE 3: VMMS Creates VM                                                   │
│  VMMS receives CreateContainer → configures VM → attaches VHDs/devices     │
│  Table: HyperVVmmsTable (azcore/Fa)                                        │
│  Check: VHD attachment, configuration errors, Level < 3 events             │
├──────────────────────────────────────────────────────────────────────────────┤
│  STAGE 4: Worker Process (vmwp.exe) Starts                                  │
│  Worker launches → VM VPs start → BIOS/UEFI executes                       │
│  Table: HyperVWorkerTable (azcore/Fa)                                      │
│  Check: EventId 18500 ("started successfully"), VmNameToIdMapping          │
│  Also: AgentOperations() function in SharedWorkspace                        │
├──────────────────────────────────────────────────────────────────────────────┤
│  STAGE 5: VTL2 / Underhill Boots (if Underhill VM)                          │
│  Linux kernel in VTL2 starts → HCL initializes → devices enumerated       │
│  Table: UnderhillEventTable (wdgeventstore/AzureHostOs)                    │
│  Check: Boot messages, device initialization, HCL events                   │
│  Also: HyperVVmConfigSnapshot for Underhill config/memory                  │
├──────────────────────────────────────────────────────────────────────────────┤
│  STAGE 6: VTL0 / Guest OS Boots                                            │
│  Guest UEFI/BIOS → OS loader → kernel → services → heartbeat IC starts    │
│  Table: VmHealthRawStateEtwTable (azcore/Fa)                               │
│  Check: VmHyperVIcHeartbeat transitions from NoContact → Ok               │
│  Also: HyperVWorkerTable for firmware boot events, EFI diagnostics         │
├──────────────────────────────────────────────────────────────────────────────┤
│  STAGE 7: VM Healthy                                                        │
│  Heartbeat IC reporting → agent provisioning → customer workload running   │
│  Table: MycroftContainerHealthSnapshot (azcore/AzureCP)                    │
│  Check: ContainerState == "ContainerStateStarted", LifecycleState          │
└──────────────────────────────────────────────────────────────────────────────┘
```

### Which Table at Each Stage — Quick Reference

| Question | Primary Table | Cluster/DB |
|----------|--------------|------------|
| Did the node boot? | `TMMgmtNodeEventsEtwTable` | gandalf/AzureCM |
| Was there a host OS update? | `AnyHostUpdateOnNode()` | wdgeventstore/HostOSDeploy |
| Is the hypervisor healthy? | `HyperVHypervisorTable` | azcore/Fa |
| Did VMMS create the VM? | `HyperVVmmsTable` | azcore/Fa |
| Did vmwp start the VM? | `HyperVWorkerTable` | azcore/Fa |
| Did Underhill boot? | `UnderhillEventTable` | wdgeventstore/AzureHostOs |
| Did the guest boot? | `VmHealthRawStateEtwTable` | azcore/Fa |
| What does the agent think? | `AgentOperations()` | azcore/SharedWorkspace |
| Is the container healthy? | `MycroftContainerHealthSnapshot` | azcore/AzureCP |

---

## 5. Cross-Table Correlation Techniques

### 5.1 The Big Union Query — All HyperV + Underhill Tables at Once

This is the single most useful query for any VM investigation. It shows a unified timeline across all five major logging sources:

```kql
// Combined timeline: Underhill + VMMS + Worker + Hypervisor + VPCI
let fn_nodeId = '<NodeId>';
let fn_containerId = '<ContainerId>';
let fn_startTime = datetime(<StartTime>);
let fn_endTime = datetime(<EndTime>);
let fn_filter = dynamic(['vmid', 'vmname', 'virtualmachineid', 'virtualmachinename',
    'fields', 'level', 'timestamp', 'op_code', 'related_activity_id', 'activity_id']);
//
// --- Underhill events (VTL2 / Linux kernel) ---
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
//
// --- VMMS events (VM management service) ---
let vmms = cluster('azcore.centralus').database('Fa').HyperVVmmsTable
    | where NodeId == fn_nodeId
    | where Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Message !contains "WHERE clause operator"
        and Message !contains "Provider could not handle query"
    | where Level <= 4
    | extend Table = "vmms";
//
// --- Worker process events (vmwp.exe) ---
let vmwp = cluster('azcore.centralus').database('Fa').HyperVWorkerTable
    | where NodeId == fn_nodeId
    | where Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Level <= 4
    | extend MessageParsed = parse_json(tolower(tostring(Message)))
    | extend Fields = bag_remove_keys(MessageParsed, fn_filter)
    | extend Message = tostring(Fields)
    | extend Table = "vmwp";
//
// --- Hypervisor events ---
let vmhv = cluster('azcore.centralus').database('Fa').HyperVHypervisorTable
    | where NodeId == fn_nodeId
    | where Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Level <= 4
    | extend Table = "vmhv";
//
// --- VPCI events (virtual PCI) ---
let vpci = cluster('azcore.centralus').database('Fa').HyperVVPciTable
    | where NodeId == fn_nodeId
    | where Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Level <= 4
    | extend MessageParsed = parse_json(tolower(tostring(Message)))
    | extend Fields = bag_remove_keys(MessageParsed, fn_filter)
    | extend Message = tostring(Fields)
    | extend Table = "vpci";
//
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

### 5.2 Timeline Reconstruction Query

Build a complete timeline for a VM across container health, agent operations, and HyperV events:

```kql
// Step 1: Container health timeline
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
let fn_startTime = datetime(<FaultTime>) - 1h;
let fn_endTime = datetime(<FaultTime>) + 1h;
cluster('azcore.centralus').database('AzureCP').MycroftContainerHealthSnapshot
| where ContainerId == fn_containerId and NodeId == fn_nodeId
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| project Tenant, PreciseTimeStamp, ContainerId, ContainerState, LifecycleState, FaultInfo
| order by PreciseTimeStamp asc
```

```kql
// Step 2: Agent operations timeline (what the agent did)
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
let fn_startTime = datetime(<FaultTime>) - 1h;
let fn_endTime = datetime(<FaultTime>) + 1h;
cluster('azcore.centralus').database('SharedWorkspace')
    .AgentOperations(fn_nodeId, fn_containerId, fn_startTime, fn_endTime)
```

```kql
// Step 3: HyperV container started check
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
let fn_startTime = datetime(<FaultTime>) - 1h;
let fn_endTime = datetime(<FaultTime>) + 1h;
cluster('azcore.centralus').database('SharedWorkspace')
    .HyperVContainerStarted(fn_nodeId, fn_containerId, fn_startTime, fn_endTime)
```

### 5.3 Joining Tables by Common Fields

The primary join pattern across tables uses `NodeId` + `ContainerId` + overlapping `PreciseTimeStamp` windows:

```kql
// Example: Join VMMS errors with Worker events for same container
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
let fn_start = datetime(<Start>);
let fn_end = datetime(<End>);
let vmms_errors = cluster('azcore.centralus').database('Fa').HyperVVmmsTable
    | where NodeId == fn_nodeId and PreciseTimeStamp between (fn_start .. fn_end)
    | where Level < 3  // Errors and Warnings
    | where EventMessage has fn_containerId
    | project PreciseTimeStamp, Source="VMMS", EventMessage;
let worker_events = cluster('azcore.centralus').database('Fa').HyperVWorkerTable
    | where NodeId == fn_nodeId and PreciseTimeStamp between (fn_start .. fn_end)
    | where Level < 3
    | where Message has fn_containerId
    | project PreciseTimeStamp, Source="Worker", EventMessage=Message;
union vmms_errors, worker_events
| order by PreciseTimeStamp asc
```

---

## 6. Investigation Flows

### 6.1 VM Won't Start (Container Start Failure)

**Prerequisites:** NodeId, ContainerId, Timestamp of failure

**Step 1: Check agent operations** — Did the agent even attempt to start the VM?
```kql
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
let fn_startTime = datetime(<FaultTime>) - 1h;
let fn_endTime = datetime(<FaultTime>) + 1h;
cluster('azcore.centralus').database('SharedWorkspace')
    .AgentOperations(fn_nodeId, fn_containerId, fn_startTime, fn_endTime)
```
Look for `StartContainer` operations and their `ResultCode`. `0x0` = success.

**Step 2: Check if Hyper-V started the VM**
```kql
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
let fn_faultTime = datetime(<FaultTime>);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
cluster('azcore.centralus').database('SharedWorkspace')
    .HyperVContainerStarted(fn_nodeId, fn_containerId, fn_startTime, fn_endTime)
```
Look for EventId 18500: `'<ContainerId>' started successfully`.

**Step 3: Check VMMS for errors** — VHD not found? Configuration error?
```kql
let fn_nodeId = "<NodeId>";
let fn_faultTime = datetime(<FaultTime>);
cluster('azcore.centralus').database("Fa").HyperVVmmsTable
| where PreciseTimeStamp between(fn_faultTime - 1h .. fn_faultTime + 1h)
    and NodeId == fn_nodeId
| where Level < 3
| where EventMessage has "<ContainerId>"
| project PreciseTimeStamp, Level, ProviderName, EventId, EventMessage
```

**Step 4: Check Storage Stack** — VHD I/O errors?
```kql
let fn_nodeId = "<NodeId>";
let fn_faultTime = datetime(<FaultTime>);
cluster('azcore.centralus').database('Fa').HyperVStorageStackTable
| where NodeId == fn_nodeId
    and PreciseTimeStamp between(fn_faultTime - 1h .. fn_faultTime + 1h)
| where Level < 3
| where Message contains "vhd"
| project PreciseTimeStamp, ProviderName, Level, EventId, Message
| order by PreciseTimeStamp asc
```

**Step 5: If Underhill VM, check UnderhillEventTable** — Did VTL2 boot?
```kql
let fn_nodeId = '<NodeId>';
let fn_containerId = '<ContainerId>';
let fn_faultTime = datetime(<FaultTime>);
cluster('wdgeventstore.kusto.windows.net').database('AzureHostOs').UnderhillEventTable
| where TIMESTAMP between (fn_faultTime - 5m .. fn_faultTime + 5m)
| where NodeId == fn_nodeId
| where VmName == fn_containerId
| where Level <= 4
| project PreciseTimeStamp, Level, TaskName, Message
| order by PreciseTimeStamp asc
```

**Step 6: Run the Big Union Query** (Section 5.1) to see complete timeline.

**Routing:** If VMMS error → Hyper-V SME. If VHD not found → Storage Client / VMService. If Underhill crash → Underhill team.

---

### 6.2 VM Unexpected Reboot

**Step 1: Check if the customer rebooted the VM** — Guest-initiated reset
```kql
// EventId 18514 = guest OS reset
let fn_faultTime = datetime(<FaultTime>);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
cluster('azcore.centralus').database("Fa").HyperVWorkerTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    and EventId == "18514"
| where NodeId == fn_nodeId
| where EventMessage has fn_containerId
| project PreciseTimeStamp, EventId, EventMessage
| sort by PreciseTimeStamp asc
```
If EventId 18514 is present → guest-initiated reset. Check timing vs. container start.

**Step 2: Check when the VM was last started**
```kql
// Container started event
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
let fn_faultTime = datetime(<FaultTime>);
cluster('azcore.centralus').database('SharedWorkspace')
    .HyperVContainerStarted(fn_nodeId, fn_containerId, fn_faultTime - 1h, fn_faultTime + 1h)
```
If reset is **> 5 minutes** after start → likely guest OS issue. Route to guest OS team.
If reset is **soon after** start → follow boot process investigation (Step 3+).

**Step 3: Check VM heartbeat** — Was the VM healthy before the reboot?
```kql
let fn_startTime = datetime(<FaultTime>) - 1h;
let fn_endTime = datetime(<FaultTime>) + 1h;
let fn_containerId = "<ContainerId>";
cluster('azcore.centralus').database('Fa').VmHealthRawStateEtwTable
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where ContainerId == fn_containerId
| project PreciseTimeStamp, ContainerId, VmHyperVIcHeartbeat, VmPowerState,
    HasHyperVHandshakeCompleted, IsVscStateOperational, Context
| sort by PreciseTimeStamp asc
| extend PrevHeartbeat = prev(VmHyperVIcHeartbeat)
| where isnull(PrevHeartbeat) or (VmHyperVIcHeartbeat != PrevHeartbeat)
    or (VmPowerState != prev(VmPowerState))
| project PreciseTimeStamp, ContainerId, VmHyperVIcHeartbeat, VmPowerState,
    HasHyperVHandshakeCompleted, IsVscStateOperational, Context
```

**Step 4: If HeartBeatStateOk during fault period** → guest issue. Route via RDOS Route.
**Step 5: If HeartBeatStateLostCommunication** → check for host-side issues. Run Big Union Query.

---

### 6.3 VM Crash / BSOD / Triple Fault

**Step 1: Check for guest bugcheck/triple fault in Worker table**
```kql
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
let fn_faultTime = datetime(<FaultTime>);
cluster('azcore.centralus').database('Fa').HyperVWorkerTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between (fn_faultTime - 30m .. fn_faultTime + 10m)
| where Message has fn_containerId
| where Message has_any ("triple fault", "bugcheck", "crash", "BSOD", "exception")
| project PreciseTimeStamp, EventId, TaskName, Message
```

**Step 2: Check if this is a Gen2 (UEFI) vs Gen1 (BIOS) VM** — affects boot diagnostics available.

**Step 3: For Underhill VMs, check UnderhillEventTable** for VTL2-side crash info.

**Step 4: Check Azure Watson** at `aka.ms/azurewatson` using NodeId and ContainerId to find crash dumps.

**Routing:**
- Guest BSOD → WSD CFE/HCCompute-Guest OS Health (Windows), LSG/Triage (Linux)
- Host bugcheck → Triage host crash TSG (Section 6.10)
- Hypervisor crash (HYPERVISOR_ERROR) → Hyper-V SME - Virtualization

---

### 6.4 VM Slow / Performance Issues

**Step 1: Check VM CPU usage**
- Refer to the VM High CPU Usage TSG. Check if the node itself has high CPU.

**Step 2: Check node-level memory pressure**
- Check available memory via host perf counters.
- Check for pool tag leaks (paged/non-paged).

**Step 3: Check if a servicing operation occurred**
```kql
// Any host update during fault window?
cluster('wdgeventstore.kusto.windows.net').database('HostOSDeploy')
    .AnyHostUpdateOnNode(
        StartTime=datetime(<FaultTime>) - 2h,
        EndTime=datetime(<FaultTime>),
        nodeList=dynamic(["<NodeId>"]))
```

**Step 4: Check for live migration impact** — was the VM being migrated?

---

### 6.5 Underhill (VTL2) Crash

**Step 1: Confirm VM is Underhill** (see Section 3, "Check if VM is Underhill")

**Step 2: Check Underhill version**
```kql
let fn_nodeId = "<NodeId>";
let fn_startTime = datetime(<FaultTime>) - 1d;
let fn_endTime = datetime(<FaultTime>) + 1h;
cluster('azcore.centralus').database('Fa').OsFileVersionTable
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where FileName == "vmfirmwareigvm.dll"
| where FileVersion != "FileNotFound"
| project PreciseTimeStamp, FileName, FileVersion, FileTimeStamp, FileSize
```

**Step 3: Check Underhill memory allocation**
```kql
let fn_nodeId = "<NodeId>";
let fn_containerId = "<ContainerId>";
let fn_startTime = datetime(<FaultTime>) - 1d;
let fn_endTime = datetime(<FaultTime>) + 1h;
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId and ContainerId == fn_containerId
| where SummaryJson contains "Vtl2RamBaseAddrOffsetMb"
| extend m = parse_json(SummaryJson)
| extend mem = parse_json(m.Memory)
| extend state = parse_json(m.VmState)
| project state.Current, mem.Vtl2RamSizeInMb, mem.Vtl2MmioSizeInMb
```

**Step 4: Check UnderhillEventTable** for crash/panic/OOM messages
```kql
let fn_nodeId = '<NodeId>';
let fn_containerId = '<ContainerId>';
let fn_faultTime = datetime(<FaultTime>);
cluster('wdgeventstore.kusto.windows.net').database('AzureHostOs').UnderhillEventTable
| where TIMESTAMP between (fn_faultTime - 10m .. fn_faultTime + 5m)
| where NodeId == fn_nodeId and VmName == fn_containerId
| where Level <= 3  // Error and Critical only
| project PreciseTimeStamp, Level, TaskName, Message
| order by PreciseTimeStamp asc
```

**Step 5: Run Big Union Query** (Section 5.1) centered on the crash time.

**Routing:** Underhill crashes → Underhill/HCL team via Hyper-V SME queue.

---

### 6.6 Live Migration Failure

**Step 1: Get the LM Session ID** (if not already in the ICM)
```kql
let fn_startTime = datetime(<FaultTime>) - 2d;
let fn_endTime = datetime(<FaultTime>) + 1h;
let fn_nodeId = "<DestinationNodeId>";
let fn_containerId = "<DestinationContainerId>";
cluster('vmainsight').database('Air').LiveMigrationFailureEvents
| where EventTime between (fn_startTime .. fn_endTime)
| extend sessionId = tostring(parse_json(Diagnostics)["SessionId"]),
    HyperVVMId = tostring(parse_json(Diagnostics)["HyperVVMId"]),
    destinationNodeId = tostring(parse_json(Diagnostics)["DestinationNodeId"]),
    destinationContainerId = tostring(parse_json(Diagnostics)["DestinationContainerId"])
| where destinationNodeId == fn_nodeId
| where destinationContainerId == fn_containerId
| project EventTime, RCALevel1, RCALevel2, sessionId,
    sourceNode = NodeId, sourceContainerId = ObjectId,
    HyperVVMId, destinationContainerId, destinationNodeId
```

**Step 2: Get source/destination NodeIds and ContainerIds**
```kql
let fn_lmSessionId = "<SessionId>";
cluster('azcore.centralus').database('Fc').LiveMigrationSessionCompleteLog
| where sessionId == fn_lmSessionId
| project sourceNodeId, sourceContainerId, destinationNodeId,
    destinationContainerId, vmUniqueId, TIMESTAMP, liveMigrationSessionId = sessionId
```

**Step 3: Run Big Union Query** on both source AND destination nodes to see the full migration flow.

**Step 4: Check `AirLiveMigrationEvents`** for brownout/blackout timing.

**Key tables for LM investigation:**
| Table | What to Look For |
|-------|-----------------|
| `LiveMigrationSessionCompleteLog` | Session outcome, source/dest IDs |
| `LiveMigrationFailureEvents` | RCA levels, failure diagnostics |
| `AirLiveMigrationEvents` | Brownout/blackout durations, AccelNet status |
| `HyperVEvents` | HyperV logging during migration |
| `HyperVVmmsTable` | VMMS errors during migration |
| `HyperVWorkerTable` | Worker process errors during migration |
| `WindowsEventsTable` | Windows events for the container |

---

### 6.7 Storage Issues

**Step 1: Check HyperVStorageStackTable** for VHD/disk errors
```kql
let fn_nodeId = "<NodeId>";
let fn_faultTime = datetime(<FaultTime>);
cluster('azcore.centralus').database('Fa').HyperVStorageStackTable
| where NodeId == fn_nodeId
    and PreciseTimeStamp between(fn_faultTime - 1h .. fn_faultTime + 1h)
| where Level < 3
| project PreciseTimeStamp, ProviderName, Level, EventId, Message
| order by PreciseTimeStamp asc
```

**Step 2: Check VMMS for VHD-related errors**
```kql
cluster('azcore.centralus').database("Fa").HyperVVmmsTable
| where PreciseTimeStamp between(datetime(<FaultTime>) - 1h .. datetime(<FaultTime>) + 1h)
| where NodeId == "<NodeId>"
| where Level < 3
| where EventMessage contains "vhd"
| project PreciseTimeStamp, Level, ProviderName, EventId, ChannelName, EventMessage
```

**Routing:**
- VHD parent not found → check configuration, route to OneFleet Node/VMService
- Bugcheck `0xba000000` → Azure Host Storage/Storage Client (blobcache driver)
- NVMe direct issues → RDOS/zHYP SME ADV via Hyper-V SME queue
- ABC/blob cache/managed disks → Storage Client team
- Storage hardware → CHIE Storage
- NTFS/ReFS/Spaces → RDOS/Azure Host OS SME - Storage (SFS)

---

### 6.8 Networking Issues

**Routing table for network issues:**

| Problem | Route To |
|---------|----------|
| Connectivity/performance/datapath issues with VMs | Host Networking\Triage |
| VFP, GFT, FPGA, Mellanox, MANA NIC issues | Host Networking\Triage |
| VM port blocked | Host Networking\Triage |
| Synthetic Ethernet port not found | Host Networking\Triage (NMAgent) |
| VM fails to start or bugcheck | RDOS/Azure Host OS SME - Virtualization (Hyper-V) |
| Host DNS resolution | Windows Platform/Core Networking - DNS Client |
| Guest networking issues | WSD Team Map |
| vSwitch issues (AccelNet, vmswitch tags) | Windows Platform/vSwitch |
| NDIS issues (DMA, binding) | Windows Platform/Core Networking - NDIS |
| TCP/UDP/IP, firewall, QUIC | Windows Platform/TCPIP/Firewall/QUIC/eBPF/XDP |
| MANA performance data | Check `netperf.kusto.windows.net / NetPerfKustoDB` |

---

### 6.9 Servicing / Deployment Failures

**Step 1: Check if any host update occurred near fault time**
```kql
cluster('wdgeventstore.kusto.windows.net').database('HostOSDeploy')
    .AnyHostUpdateOnNode(
        StartTime=datetime(<FaultTime>) - 2h,
        EndTime=datetime(<FaultTime>),
        nodeList=dynamic(["<NodeId>"]))
```

**Step 2: Check pilotfish package deployments**
```kql
let node_list = dynamic(["<NodeId1>", "<NodeId2>"]);
let fault_time = datetime(<FaultTime>);
cluster('azdeployer.kusto.windows.net').database('AzDeployerKusto').ServiceVersionSwitch
| where PreciseTimeStamp between (fault_time - 2h .. fault_time)
| where NodeId in~ (node_list)
| where CurrentVersion != ''
| project NodeId_Azure, PreciseTimeStamp, ServiceName, CurrentVersion, NewVersion
```

**Step 3: Check virtual environment updates**
```kql
let node_list = dynamic(["<NodeId1>", "<NodeId2>"]);
let fault_time = datetime(<FaultTime>);
cluster('azdeployer.kusto.windows.net').database('AzDeployerKusto').OMWorkerRepairGenerator
| where PreciseTimeStamp between (fault_time - 2h .. fault_time + 2h)
| where azureNodeId in~ (node_list)
| project azureNodeId, PreciseTimeStamp, azurePENodeState, virtualEnvironment,
    cluster, environment, actionRequired, assignedVersionTimestamp,
    assignedVersion, expectedVersion, dmMachineState
| where azurePENodeState != 'Ready'
| where todatetime(assignedVersionTimestamp) between (PreciseTimeStamp - 30m .. PreciseTimeStamp)
| where assignedVersion != ''
```

**Routing:**
- RDOS Host OS updates → HostUpdate@microsoft.com / RDOS/Azure Host OS SME - Update Technologies
- HCM issues → HCMTriage@microsoft.com
- Other pilotfish deployments → respective agent team DRI

---

### 6.10 Node Reboot / Host Crash

**Step 1: Determine why nodes are rebooting** (aggregate by fault reason)
```kql
let fn_tenant = "<TenantName>";  // e.g., "CDM03PrdApp04"
cluster('hawkeyedataexplorer.centralus.kusto.windows.net').database('HawkeyeLogs')
    .GetLatestFaultedNodes(fn_tenant)
| where HawkeyeOutput !contains "Tip"  // Exclude TiP nodes
| where faultInfo contains "reboot"
| summarize count() by FaultReason
| sort by count_ desc
```

**Step 2: Check if reboots are due to bugchecks**
```kql
let fn_tenant = "<TenantName>";
let fn_faultReason = "<FaultReasonFromStep1>";
cluster('hawkeyedataexplorer.centralus.kusto.windows.net').database('HawkeyeLogs')
    .GetLatestFaultedNodes(fn_tenant)
| where HawkeyeOutput !contains "Tip"
| where faultInfo contains "reboot"
| where FaultReason contains fn_faultReason
| where faultInfo contains "bugcheck"
```

**Step 3: Check node event history**
```kql
let fn_nodeId = "<NodeId>";
let fn_reproTime = datetime(<FaultTime>);
let start_time = fn_reproTime - 2h;
let end_time = fn_reproTime;
cluster("gandalf").database("AzureCM").TMMgmtNodeEventsEtwTable
| where PreciseTimeStamp between (start_time .. end_time)
| where NodeId in~ (fn_nodeId)
| project TIMESTAMP, Message
```

**Step 4: Check Hawkeye RCA**
```kql
let fn_resourceId = "<NodeId>";
cluster("hawkeyedataexplorer.centralus.kusto.windows.net").database("HawkeyeLogs")
    .HawkeyeRCAEvents
| where ResourceId in (fn_resourceId)
| summarize arg_max(PreciseTimeStamp, *) by ResourceId, Scenario, NodeId
| project FaultTime, Scenario, RCALevel1, RCALevel2, EscalateToTeam, AdditionalDetails
```

**Step 5: For crashes — triage dumps**
- Go to `aka.ms/azurewatson` with NodeId/ContainerId
- Find faulting module → look up in ownership tool
- Route using the OS Area Path → SME Queue mapping (Section 7)

**Routing:**
- OS_DHCP_Not_Found → Host boot failure investigation (boot logs)
- HYPERVISOR_ERROR → Hyper-V SME - Virtualization
- Anvil NMI (nt!HalHandleNMI frames) → Check Anvil-initiated NMIs
- No dump available → RDOS/Azure Host OS - Rainier Automation
- Critical SvcHost crash (0xEF) → See SvcHost crash triage (check exit codes for OOM)

---

### 6.11 VM Creation Failure (SLA Drop)

**Step 1: If you have NodeId + ContainerId + Timestamp** → follow Start Container Failure flow (Section 6.1)

**Step 2: Check VMMS for VHD errors**
```kql
let fn_nodeId = "<NodeId>";
let fn_faultTime = datetime(<FaultTime>);
cluster('azcore.centralus').database("Fa").HyperVVmmsTable
| where PreciseTimeStamp between(fn_faultTime - 1h .. fn_faultTime + 1h)
    and NodeId == fn_nodeId
| where Level < 3
| where EventMessage contains "vhd"
| project PreciseTimeStamp, Level, ProviderName, EventId, ChannelName, EventMessage
```

**Step 3: Check EG links** — if the incident includes EG (Execution Graph) links, use the EG Viewer to identify which stage failed.

**Routing:** TDPR issues → Provisioning Agent team first. Route to RDOS only if confirmed Host OS issue.

---

## 7. Incident Triage — Team Ownership

### 7.1 Quick Routing Decision Tree

```
Is this RDOS / Host OS related?
├─ Check the Applicability Guide (aka.ms/rdosra)
├─ Run AnyHostUpdateOnNode() — was there a host update?
├─ Check Hawkeye RCA — does it point to a specific team?
│
├─ YES → Continue investigation
│   ├─ Hypervisor bugcheck (HYPERVISOR_ERROR) → Hyper-V SME - Virtualization
│   ├─ VM won't start / vmwp crash → Hyper-V SME - Virtualization
│   ├─ Underhill / HCL issue → Hyper-V SME - Virtualization
│   ├─ Storage / VHD / NVMe → Storage (SFS) or Storage Client
│   ├─ Networking → Host Networking\Triage
│   ├─ Memory management / scheduling → Kernel (BK)
│   ├─ Boot / PXE / KSR → Secure Platform (SPF)
│   ├─ PCI / ACPI / power → Platform (BPT)
│   ├─ Servicing deployment → Update Technologies
│   ├─ ETW / WER / perf counters → ETW/WER
│   ├─ TLS / certs / TPM → Security (ENS)
│   ├─ GPU / NPU / XPU → Heterogenous Compute Kernel SME
│   ├─ No dump, inconclusive → Rainier Automation
│   └─ Unknown vmwp component → Further investigation by Hyper-V SME
│
└─ NO → Route to correct team via aka.ms/rdosroute
    ├─ Guest OS (Windows) → WSD CFE/HCCompute-Guest OS Health
    ├─ Guest OS (Linux) → LSG/Triage
    ├─ Azure Stack HCI → Not RDOS (HCI team)
    ├─ TiP node issues → TiP session owner
    └─ Gandalf misroute → Tag HYP-Auto-Misroute, return to Gandalf
```

### 7.2 OS Area Path → SME Queue Mapping

| OS Area Path | SME Queue |
|-------------|-----------|
| OS\Core\Base\BK - Base Kernel | RDOS\Azure Host OS SME - Kernel (BK) |
| OS\Core\Base\BPT - Base Platform Technologies | RDOS\Azure Host OS SME - Platform (BPT) |
| OS\Core\Base\SPF - Secure Platform Foundation | RDOS\Azure Host OS SME - Secure Platform (SPF) |
| OS\Core\Base\VCP - Virtualization Core Platform | RDOS\Azure Host OS SME - Virtualization (Hyper-V) |
| OS\Core\Base\VMAC - Virtual Machines and Containers | RDOS\Azure Host OS SME - Virtualization (Hyper-V) |
| OS\Core\ENS | RDOS\Azure Host OS SME - Security (ENS) |
| OS\Core\Fundamentals\Diagnostics\Instrumentation | RDOS\Azure Host OS SME - ETW/WER |
| OS\Core\LIOF\SFS - Storage and File Systems | RDOS\Azure Host OS SME - Storage (SFS) |
| OS\Core\SiGMa\GRFX - Graphics | RDOS\Heterogenous Compute Kernel SME |

### 7.3 Top-Level Incident Queues

| Queue | When to Use |
|-------|------------|
| RDOS\Azure Host OS DRI - Sev 1-2 | High severity host OS incidents |
| RDOS\Azure Host OS DRI - Sev 3-4 | Low severity host OS incidents |
| RDOS\Azure Host OS SME - Update Technologies | Servicing / deployment issues |
| RDOS\Azure Host OS - Rainier Automation | Unexplained reboots without dumps |

### 7.4 ICM Custom Fields

Always update ICM custom fields for Service RDOS or add a Custom RDOS Tag. This enables proper tracking and reporting.

**Key ICM categories to search for in TSGs:**

| Category | Keywords |
|----------|----------|
| Start/Stop Container | Start container, Stop container, ContainerCreateStartRCA |
| OOM Failures | Out Of Memory, Memory saturation, OOM |
| Node Reboot | FaultReason, bugcheck, unexpected reboot |
| Node Failure | Node fault, unhealthy nodes, HI, OFR |
| Host Boot | Boot, PXE boot, UEFI, DHCP, WDS, SEL |
| VM Creation | VM Creation Failure, SLA drop |
| VM Unhealthy | Container health, VM unhealthy, Heartbeat |
| Live Migration | LM failure, Migrate-To-Suspended, Brownout, Blackout |
| Host Crashes | Host crash, Watson dump, bugcheck |
| Deployment | Host OS deployment, OneDeploy, AzDeployer, PilotFish |
| Memory Leaks | Paged/non-paged pool tags, usermode leak, OOM |
| Hibernate | Hibernate, S4 sleep state |

---

## 8. Tools and Dashboards

| Tool | URL / How to Access | Purpose |
|------|-------------------|---------|
| **RDOS RA Tool** | Internal tool (runs basic checks against ICM) | Automated routing checks, links to Watson/Node Story |
| **Azure Service Insights (ASI) / Host Analyzer** | Internal | Explore node state, HyperV events |
| **Node Story** | Internal | Node event timeline |
| **Node View** | `aka.ms/nodeview` | Quick node info |
| **Azure Host OS App** | Internal | Host state overview (replaces Node Explorer) |
| **Azure Watson** | `aka.ms/azurewatson` | Crash dump search and analysis |
| **Hawkeye** | Via Kusto | Automated RCA for faults |
| **whyfaulted dashboard** | Internal | Why a node faulted |
| **EG Viewer** | Desktop app | Execution Graph for VM creation stages |
| **DCM Explorer** | Requires Azure-Reddog-DialToneOnly-RO | Node management operations |
| **Ownership Tool** | Internal | Map faulting module → owning team |
| **aka.ms/rdosroute** | RDOS Routing Table | Find correct team for incident transfer |
| **aka.ms/rdosra** | RDOS RA Guide | Guide for teams requesting RDOS assistance |
| **aka.ms/whyunhealthy** | Internal | Why is a node/VM unhealthy |
| **Fleet Diagnostics** | Internal | Fleet-wide diagnostics authoring |

### Key Dashboards (pre-built Kusto)

- **VM has Started but is Unhealthy or Unexpected VM reboot Dashboard** — contains all queries from Section 6.2
- **Live Migration Failure Dashboard** — contains all queries from Section 6.6

---

## 9. Quick Reference: Common Queries

### Container Health Snapshot
```kql
cluster('azcore.centralus').database('AzureCP').MycroftContainerHealthSnapshot
| where ContainerId == "<ContainerId>" and NodeId == "<NodeId>"
| where PreciseTimeStamp between (datetime(<Start>) .. datetime(<End>))
| project Tenant, PreciseTimeStamp, ContainerId, ContainerState, LifecycleState, FaultInfo
| order by PreciseTimeStamp asc
```

### VM Heartbeat Check
```kql
cluster('azcore.centralus').database('Fa').VmHealthRawStateEtwTable
| where PreciseTimeStamp between (datetime(<Start>) .. datetime(<End>))
| where ContainerId == "<ContainerId>"
| project PreciseTimeStamp, ContainerId, VmHyperVIcHeartbeat, VmPowerState,
    HasHyperVHandshakeCompleted, IsVscStateOperational, Context
| sort by PreciseTimeStamp asc
```

### Underhill Events (UnderhillEventTable with parsing)
```kql
let fn_filter = dynamic(['vmid', 'vmname', 'fields', 'message', 'level',
    'timestamp', 'op_code']);
let fn_filter2 = dynamic(['name', 'target', 'time_taken_ns', 'time_active_ns',
    'activity_id', 'related_activity_id', 'correlationid', 'correlation_id']);
cluster('wdgeventstore.kusto.windows.net').database('AzureHostOs').UnderhillEventTable
| where TIMESTAMP between (datetime(<Start>) .. datetime(<End>))
| where NodeId == "<NodeId>"
| where VmName == "<ContainerId>"
| extend MessageParsed = parse_json(tolower(tostring(Message)))
| extend InnerMessageParsed = parse_json(tolower(tostring(MessageParsed.message)))
| extend Fields = bag_merge(MessageParsed, InnerMessageParsed)
| extend Fields = bag_remove_keys(Fields, fn_filter)
| extend Fields = bag_merge(Fields, InnerMessageParsed.fields, MessageParsed.fields)
| extend name = Fields.name
| extend target = Fields.target
| extend time_taken_ns = Fields.time_taken_ns
| extend correlation_id = iff(
    Fields.correlation_id != '00000000-0000-0000-0000-000000000000',
    Fields.correlation_id, Fields.correlationid)
| extend Fields = bag_remove_keys(Fields, fn_filter2)
| project PreciseTimeStamp, Level, TaskName, name, target,
    Fields = tostring(Fields), time_taken_ns, correlation_id
```

> **Note:** Known benign error messages in UnderhillEventTable (safe to ignore):
> - `kmsg: "Error: No information about IO-APIC in OF."`
> - `kmsg: "Cannot find an available gap in the 32-bit address range"`
> - `kmsg: "PCI devices with unassigned 32-bit BARs may not work!"`
> - `kmsg: "RETBleed: WARNING: Spectre v2 mitigation leaves CPU vulnerable..."`
> - `kmsg: "PCI: Fatal: No config space access function found"`

### Underhill Clusters Discovery
```kql
VmServiceContainerOperations
| where PreciseTimeStamp > ago(7d)
| where IsUnderhillLocalEnabled == true
    or IsUnderhillNetworkEnabled == true
    or IsUnderhillRemoteEnabled == true
| summarize dcount(NodeId) by Cluster
```

### Node Event History
```kql
let fn_nodeId = "<NodeId>";
let fn_reproTime = datetime(<FaultTime>);
cluster("gandalf").database("AzureCM").TMMgmtNodeEventsEtwTable
| where PreciseTimeStamp between (fn_reproTime - 2h .. fn_reproTime + 1h)
| where NodeId in~ (fn_nodeId)
| project TIMESTAMP, Message
```

### Any Host Update on Node
```kql
cluster('wdgeventstore.kusto.windows.net').database('HostOSDeploy')
    .AnyHostUpdateOnNode(
        StartTime=datetime(<Start>),
        EndTime=datetime(<End>),
        nodeList=dynamic(["<NodeId>"]))
```

### Hawkeye RCA Lookup
```kql
let fn_resourceId = "<NodeId_or_ContainerId>";
cluster("hawkeyedataexplorer.centralus.kusto.windows.net").database("HawkeyeLogs")
    .HawkeyeRCAEvents
| where ResourceId in (fn_resourceId)
| summarize arg_max(PreciseTimeStamp, *) by ResourceId, Scenario, NodeId
| project FaultTime, Scenario, RCALevel1, RCALevel2, EscalateToTeam, AdditionalDetails
```

### Hypervisor Event Log
```kql
let fn_nodeId = "<NodeId>";
let fn_faultTime = datetime(<FaultTime>);
cluster('azcore.centralus').database('Fa').HyperVHypervisorTable
| where PreciseTimeStamp between (fn_faultTime - 1d .. fn_faultTime + 1d)
| where NodeId == fn_nodeId
| project PreciseTimeStamp, TaskName, Message, Opcode
```

### Pilotfish Deployment Check
```kql
let node_list = dynamic(["<NodeId>"]);
let fault_time = datetime(<FaultTime>);
cluster('azdeployer.kusto.windows.net').database('AzDeployerKusto').ServiceVersionSwitch
| where PreciseTimeStamp between (fault_time - 2h .. fault_time)
| where NodeId in~ (node_list)
| where CurrentVersion != ''
| project NodeId_Azure, PreciseTimeStamp, ServiceName, CurrentVersion, NewVersion
```

---

> **Source documentation:** Distilled from RDOS Livesite EngHub pages including:
> - [TSG Overview](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/tsg-overview)
> - [Playbook Skeleton](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/playbook/skeleton)
> - [Underhill Kusto Queries FAQ](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-kusto-queries-faq)
> - [VM Unhealthy / Unexpected Reboot TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/running-container-unhealthy)
> - [Live Migration Failure TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/migration/migration-failure)
> - [Node Reboot Investigation](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite-fr/tsg/incident/node_reboot_investigation)
> - [VM Creation Failure](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite-fr/tsg/incident/vm_creation_failure_investigation)
> - [Virt Playbook](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite-fr/tsg/virtualization/virt-playbook)
> - [Host Crash Triage](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite-fr/tsg/incident/triage_host_crash)
> - [Host OS Incident Routing](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite-fr/playbook/icmcategories/osareapathmapping)
> - [Hypervisor TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite-fr/tsg/keywordmapping)
