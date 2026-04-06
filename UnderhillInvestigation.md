# Underhill Investigation - RDOS Livesite Knowledge Base

> Auto-generated from RDOS Livesite EngHub documentation traversal.
> This document captures all Underhill-related knowledge found across the livesite docs.
> Sources: RDOS Livesite TSGs, Playbook, HowTos, and related EngHub pages.

---

## Table of Contents
- [1. What is Underhill?](#1-what-is-underhill)
- [2. How to Check if a VM is Underhill](#2-how-to-check-if-a-vm-is-underhill)
- [3. Key Kusto Tables and Clusters](#3-key-kusto-tables-and-clusters)
- [4. Helpful Kusto Queries](#4-helpful-kusto-queries)
- [5. Underhill Version / Host OS Versions](#5-underhill-version--host-os-versions)
- [6. UEFI / Firmware Details](#6-uefi--firmware-details)
- [7. VM Heartbeat / Keepalive](#7-vm-heartbeat--keepalive)
- [8. Start Container Failures](#8-start-container-failures)
- [9. Stop Container Failures](#9-stop-container-failures)
- [10. Live Migration Failures](#10-live-migration-failures)
- [11. Running Container Unhealthy / Unexpected Reboots](#11-running-container-unhealthy--unexpected-reboots)
- [12. HCL (Host Compatibility Layer)](#12-hcl-host-compatibility-layer)
- [13. Known Issues (Historical)](#13-known-issues-historical)
- [14. Incident Tagging and Routing](#14-incident-tagging-and-routing)
- [15. Tools and Resources](#15-tools-and-resources)
- [16. Underhill TSG (Dedicated)](#16-underhill-tsg-dedicated)
- [17. Underhill Kusto Queries & FAQ](#17-underhill-kusto-queries--faq)
- [18. Underhill Out of Memory (OOM)](#18-underhill-out-of-memory-oom)
- [19. Underhill Servicing](#19-underhill-servicing)
- [20. Underhill Networking](#20-underhill-networking)
- [21. Underhill Specific Incident: ICM 676743226](#21-underhill-specific-incident-icm-676743226)
- [22. VMGS TSG](#22-vmgs-tsg)
- [23. Watchdog Timeouts](#23-watchdog-timeouts)
- [24. EfiDiagnostics](#24-efidiagnostics)

---

## 1. What is Underhill?

Underhill is the firmware/OS running inside VTL2 (Virtual Trust Level 2) of Azure VMs. It provides device emulation and management for guest virtual machines. It is part of the Host Compatibility Layer (HCL) stack. Underhill VMs use HCL for device emulation inside the guest.

Key characteristics:
- Runs in guest VTL2 (the secure partition)
- Provides device emulation for guest VMs
- Used by Trusted Launch (TL) and Confidential VM (CVM) scenarios
- Has its own event table in Kusto: `UnderhillEventTable`
- The `IsUnderhill` flag in VM configuration indicates whether a VM uses Underhill

Host OS versions that reference Underhill: RS1.65, RS1.8, RS1.85, RS1.86, AH2020, AH2021, AH2022, AH2023

---

## 2. How to Check if a VM is Underhill

**Use the `HyperVVmConfigSnapshot` table** to determine if a VM is an Underhill VM. Check the `IsUnderhill` column or parse it from `SummaryJson`:

```kql
let fn_nodeId = "<NODE_ID>";
let fn_containerId = "<CONTAINER_ID>";
let fn_faultTime = datetime(2023-08-27 00:00:00);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
cluster("azcore.centralus").database("Fa").HyperVVmConfigSnapshot 
| where NodeId == fn_nodeId and ContainerId == fn_containerId 
  and PreciseTimeStamp between(fn_startTime .. fn_endTime)
| where SummaryType == "Configuration"
| extend IsUnderhillFromJson = parse_json(SummaryJson).Settings.hcl.IsUnderhill
| project PreciseTimeStamp, ContainerId, NodeId, VmVersion, VmGeneration, 
  VmProcessorCount, VmMemoryInMB, IsolationSetting, HclEnabled, IsUnderhill, 
  IsUnderhillFromJson
| order by PreciseTimeStamp desc
| take 1
```

**Output fields:**
- `IsUnderhill`: Boolean - True means this is an Underhill VM
- `IsUnderhillFromJson`: Parsed from SummaryJson.Settings.hcl.IsUnderhill
- `HclEnabled`: Whether HCL is enabled (typically true for Underhill VMs)
- `IsolationSetting`: Isolation type (e.g., 16)
- `VmVersion`, `VmGeneration`: VM configuration version and generation

**Source:** StopContainer failure TSG, Step 9

---

## 3. Key Kusto Tables and Clusters

### Primary Cluster and Database
- **Cluster:** `azcore.centralus.kusto.windows.net`
- **Database:** `Fa`

### Key Tables

| Table | Description | Cluster/Database |
|-------|-------------|------------------|
| `UnderhillEventTable` | Events and traces from guest VTL2 (Underhill) | azcore.centralus / Fa |
| `HyperVVmConfigSnapshot` | VM configuration snapshots, includes IsUnderhill flag | azcore.centralus / Fa |
| `HyperVEvents` | Logging from multiple Hyper-V components | azcore.centralus / Fa |
| `HyperVVmmsTable` | VMMS logging, LM logging and errors | azcore.centralus / Fa |
| `HyperVWorkerTable` | Worker process logging, LM operations | azcore.centralus / Fa |
| `WindowsEventsTable` | Windows events for containers | azcore.centralus / Fa |
| `VmHealthRawStateEtwTable` | VM health/heartbeat state | azcore.centralus / Fa |
| `MycroftContainerHealthSnapshot` | Container health, lifecycle state, fault info | azcore.centralus / AzureCP |
| `HawkeyeRCAEvents` | RCA (Root Cause Analysis) events | hawkeyedataexplorer.westus2 / HawkeyeLogs |
| `LiveMigrationSessionCompleteLog` | Live migration session logs | — |
| `AirLiveMigrationEvents` | Detailed LM session info | — |
| `ServiceVersionSwitch` | Pilotfish package deployments | azdeployer.kusto.windows.net / AzDeployerKusto |
| `OMWorkerRepairGenerator` | Virtual environment deployment info | azdeployer.kusto.windows.net / AzDeployerKusto |

### Other Kusto Clusters Used
- `hawkeyedataexplorer.westus2.kusto.windows.net` / HawkeyeLogs
- `wdgeventstore.kusto.windows.net` / HostOSDeploy
- `azdeployer.kusto.windows.net` / AzDeployerKusto
- `baseplatform.westus.kusto.windows.net` / vmphu
- `xstore.kusto.windows.net` (Host Analyzer)
- `icmcluster.kusto.windows.net` (IcM queries)
- `hostosdata.centralus.kusto.windows.net` (RDOS RA tool)
- `sparkle.eastus.kusto.windows.net` (RDOS RA tool)
- `gandalffollower.centralus` (Gandalf - Albus Viewer)

---

## 4. Helpful Kusto Queries

### Check if Any Host OS Update on a Node
```kql
cluster('wdgeventstore.kusto.windows.net').database('HostOSDeploy').
AnyHostUpdateOnNode(
  StartTime=datetime(2025-08-10T15:38:27Z),
  EndTime=datetime(2025-08-11T15:38:27Z),
  nodeList=dynamic(["183715db-c0e2-68d0-bb55-8a79425d695c"])
)
```
**Use case:** Check if an RDOS host OS update happened during time of fault.

### Identify Pilotfish Package Deployed During Fault
```kql
let node_list = dynamic(["<NODE_IDS>"]);
let fault_time = datetime(2025-09-13 6:05:08);
cluster('azdeployer.kusto.windows.net').database('AzDeployerKusto').ServiceVersionSwitch
| where PreciseTimeStamp between (fault_time-2h .. fault_time)
| where NodeId in~ (node_list)
| where CurrentVersion != ''
| project NodeId_Azure, PreciseTimeStamp, ServiceName, CurrentVersion, NewVersion
```

### Identify Virtual Environment Deploying Pilotfish During Fault
```kql
let node_list = dynamic(["<NODE_IDS>"]);
let fault_time = datetime(2025-09-13 6:05:08);
cluster('azdeployer.kusto.windows.net').database('AzDeployerKusto').OMWorkerRepairGenerator
| where PreciseTimeStamp between (fault_time-2h .. fault_time+2h)
| where azureNodeId in~ (node_list)
| project azureNodeId, PreciseTimeStamp, azurePENodeState, virtualEnvironment, cluster, environment, actionRequired, assignedVersionTimestamp, assignedVersion, expectedVersion, dmMachineState
| where azurePENodeState != 'Ready'
| where todatetime(assignedVersionTimestamp) between (PreciseTimeStamp-30m .. PreciseTimeStamp)
| where assignedVersion != ''
| project azureNodeId, PreciseTimeStamp, azurePENodeState, virtualEnvironment, cluster, environment, assignedVersionTimestamp
```

### Hawkeye RCA Events (Root Cause Analysis)
```kql
let fn_nodeId = "<NODE_ID>";
cluster('hawkeyedataexplorer.westus2').database('HawkeyeLogs').HawkeyeRCAEvents
| where NodeId == fn_nodeId
| where Scenario contains "Container"
| project NodeId, FaultTime, Scenario, RCALevel1, RCALevel2, EscalateToTeam, EscalateToOrg
```

### Container Health Snapshot (Fault Info)
```kql
let fn_nodeId = "<NODE_ID>";
let fn_containerId = "<CONTAINER_ID>";
let fn_faultTime = datetime(2023-05-22T18:37:14Z);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
cluster('azcore.centralus').database('AzureCP').MycroftContainerHealthSnapshot
| where ContainerId == fn_containerId
| where PreciseTimeStamp between (fn_startTime .. fn_endTime)
| where LifecycleState in ("ToBeDestroyedOnNode","Destroyed")
| project Tenant, PreciseTimeStamp, ContainerId, ContainerState, LifecycleState, FaultInfo
| order by PreciseTimeStamp asc
```

---

## 5. Underhill Version / Host OS Versions

Host OS versions referenced in livesite documentation:
- RS1.65, RS1.8, RS1.85, RS1.86
- AH2020, AH2021, AH2022, AH2023, AH2023 SP1

*More detailed version querying information pending from dedicated Underhill TSG pages.*

---

## 6. UEFI / Firmware Details

### UEFI Watchdog Timer
- **2-minute timer** starts when exiting PEI (Pre-EFI Initialization) phase
- For each bootable device, UEFI loads the image and starts a **5-minute timer** if successful
- The guest OS is expected to **disable the watchdog timer** when it boots successfully
- Watchdog firing indicates guest hasn't progressed far enough into boot to disable timer
- Issue could be in UEFI itself, VM configuration, or guest OS

### UEFI Event IDs
| Event ID | Description |
|----------|-------------|
| 18600 | UEFI watchdog timeout and VM reset |
| 18601 | Successfully booted an operating system (Gen2 VMs only) |
| 18602 | Guest crash detected |
| 18603 | Boot failure |
| 18604 | Guest crash detected (variant) |
| 18605 | Boot event |
| 18606 | Boot attempt event (Gen1 VMs, starting AH2023) |
| 18610 | Fatal virtual firmware error (HCL-specific) |
| 18514 | VM was reset by guest OS |
| 18540 | Triple fault detected |
| 18550 | Triple fault detected (variant) |
| 18560 | Triple fault detected (variant) |
| 18590 | Guest crash detected |

### Generation-Specific Notes
- **Generation 1 VMs:** Don't have successful boot event, starting in AH2023 they log boot attempt event (18606)
- **Generation 2 VMs:** Have successful boot events including event ID 18601

---

## 7. VM Heartbeat / Keepalive

VM health is monitored through the **Heartbeat IC (Integration Component)**:
- Table: `VmHealthRawStateEtwTable`
- Used to check if a VM is healthy after starting
- Heartbeat loss indicates VM is unhealthy or has become unresponsive
- Key investigation point for "VM has Started but Unhealthy" scenarios

---

## 8. Start Container Failures

**TSG Source:** StartContainer failure / VM start failure (or timeout) / Slow Start

Key investigation areas:
- Check Hawkeye RCA events
- Check container health snapshots
- Check if VM is Underhill (Step 9 in StopContainer, similar check for Start)
- Check UEFI boot events for Gen1 vs Gen2 VMs
- Memory availability on node
- HCL fault events (Event 18610)

Common causes:
- Out of Memory on node
- UEFI watchdog timeout
- HCL/Underhill initialization failure
- Driver/firmware incompatibility
- Bitlocker/vTPM issues

---

## 9. Stop Container Failures

**TSG Source:** TSG: StopContainer failure / VM stop failure (or timeout)

### Step 9: Is the VM Underhill?
This is an explicit step in the StopContainer failure TSG. If the VM is Underhill, the TSG directs you to the dedicated Underhill TSG.

### Key queries for StopContainer:
1. Check Hawkeye RCA Events
2. Query MycroftContainerHealthSnapshot for fault info
3. Check VDEV operations
4. Check storage stack
5. Check shutdown events
6. **Check if VM is Underhill** (using HyperVVmConfigSnapshot)

### Sample Incidents:
- **ICM 291024540**: "VM stop stuck because of emulated IO storage stuck in lower layer storage component" - StopContainer timeout (1200000 ms = 20 min)
- **ICM 297511656**: "VM stop takes over 10 minutes because clean shutdown initiated and guest does not accommodate"

---

## 10. Live Migration Failures

### Live Migration Phases
- **Brownout phase:** Catching up data between source and destination (can take up to 9 hours for large VMs)
- **Blackout phase:** VM paused for final transfer
- **Point of no return:** When VM starts on destination (can't resume on source)
- **Migrate-To-Suspended:** VMs paused on destination after migration

### Key Tables for LM Investigation
- `LiveMigrationSessionCompleteLog`
- `AirLiveMigrationEvents` (brownout, compute blackout, network blackout, port programming delays)
- `HyperVEvents`
- `HyperVVmmsTable`
- `HyperVWorkerTable`
- `WindowsEventsTable`

### Common LM Issues
1. **Guest VM Unexpectedly Reboots During LM** - Event 18514 (VM reset by guest OS)
2. **Memory Transfer Timeout** - Error 800705B4 (E_TIMEOUT) and 80048054 (VM_E_MIGRATION_MSG_PEER_ABORTED)
3. **Failed VM State Change to Migrating Suspended** - VmBus power off failure, VPCI deadlock

### ID Relationships
- ContainerId = VmName from Hyper-V perspective
- VmID = GUID representing the same VM
- VmUniqueId = Azure concept for VM resource ID

---

## 11. Running Container Unhealthy / Unexpected Reboots

### Investigation Steps
1. Check container started successfully
2. Check VM heartbeat (VmHealthRawStateEtwTable)
3. Check UEFI boot events
4. Check for guest crashes (Events 18514, 18590, 18602, 18604)
5. Check for triple faults (Events 18540, 18550, 18560)
6. Check for HCL faults (Event 18610)
7. Check Bitlocker status

### Root Causes
- UEFI watchdog timeout (2-min or 5-min timer)
- Guest OS crash (bugcheck)
- Triple fault (very bad guest state)
- HCL/firmware fatal error
- Bitlocker/vTPM corruption
- Memory issues

---

## 12. HCL (Host Compatibility Layer)

- Used by **Trusted Launch (TL)** and **Confidential VM (CVM)**
- Both use HCL for device emulation inside guest
- HCL runs Underhill in VTL2
- **Event 18610**: Fatal virtual firmware error (HCL-specific fault indicator)
- Can cause guest start issues especially when previously working

---

## 13. Known Issues (Historical)

### Issues with Clusters/SKUs (from Older Known Issues page)

1. **SN4PrdApp27** - Discontinued HW (Intel S1200SPL), incompatible with AH2021
2. **Bad cluster buildout** - Unsupported HW on BaseOS (AMD Gen8 on RS1.86)
3. **CDM06PrdApp14, CHI25PrdApp15, CHI25PrdApp18** - Wrong HBA firmware
4. **AMS25PrdApp29** - M-Series reboot with CORRUPT_MODULELIST_0x80 after NMI
5. **Low Memory on specific HostGenIds** - Container start failures
6. **M-Series VM Memory Transfer Timeout** - Reboots during LM
7. **IAD01PrdHPC02** - Start container failures from low memory
8. **Gen 8 nodes** - DPC_TIMEOUT_FPGA_INVALID_REGISTERS crashes
9. **CDZ08PrdApp02** - SNP Mixed Mode issues with AH2023 SP1
10. **AZAP Bing clusters** - Out of memory due to incorrect buildout or NUMA affinity issues

### Driver/Software Issues
- Mellanox memory leak causing node crashes, VM start failures
- FPGA network driver handle leak (gftlwf.sys)
- Code sign policy violation (iomemory_vsl.sys)
- PCI DMA adapter bugcheck during AH2021 update
- Host OS crash in ci.dll (policy version incompatibility)
- Trusted Launch VMs black screen / boot failure
- SMAPI disk handle leak causing CreateContainerComplete failures

---

## 14. Incident Tagging and Routing

*(Pending - data from incident-tagging page)*

---

## 15. Tools and Resources

### RDOS RA Tool
- Runs basic checks against IcM
- Runs same queries as routing bot
- Attempts to determine if incident is related to RDOS
- Provides links to Watson crashes, Node Story, ASI
- Node IDs format: `["c4b7c8df-8f47-2372-3be5-233287006155", "abcdefab-..."]`
- Contact: hostosarbot@microsoft.com

### Other Tools
- **aka.ms/whyunhealthy** - Check why a node is unhealthy
- **aka.ms/nodeview** - Node view tool
- **Host Analyzer** - Uses xstore.kusto.windows.net
- **DCM.Explorer** - Requires Azure-Reddog-DialToneOnly-RO membership
- **ANA (Azure Node Automation)** - For creating tip sessions
- **Fleet Diagnostics** - Access/author fleet diagnostics
- **PFGold** - Azure-Gold-Config
- **OaaS (Orchestration-as-a-Service)** - Uses Cosine VE

### Useful aka.ms Links
- `aka.ms/rdosra` - RDOS RA guide
- `aka.ms/whyunhealthy` - Node health check
- `aka.ms/nodeview` - Node view
- `aka.ms/azurecen` - Azure severity mappings
- `aka.ms/hostosservicing` - Servicing bugs info

---

---

## 16. Underhill TSG (Dedicated)

**Source:** https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-tsg

### Prerequisites
- Node ID
- Container ID
- Timestamp of the fault

### Step 1: Determine if the VM is an Underhill VM

```kql
let fn_nodeId = "d71bdb10-080b-705a-ed75-568665161908";
let fn_containerId = "54eb2fa6-80e5-4ac4-82ac-ad3e19d160b2";
let fn_faultTime = datetime(2024-04-24T02:33:08Z);
let fn_startTime = fn_faultTime-1d;
let fn_endTime = fn_faultTime+1h;
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot
| where NodeId == fn_nodeId and ContainerId == fn_containerId and PreciseTimeStamp between(fn_startTime .. fn_endTime)
| where SummaryType == "Configuration"
| extend IsUnderhillFromJson = parse_json(SummaryJson).Settings.hcl.IsUnderhill
| project PreciseTimeStamp, IsUnderhill = iff(isnotempty(IsUnderhill), IsUnderhill, IsUnderhillFromJson)
| order by PreciseTimeStamp desc
| take 1
```

> **Note:** The HyperVVmConfigSnapshot is only populated for hosts running **AH2023 and newer**. Older host OSes don't log information to this table. If no rows are returned, check the UnderhillVersion or ask the VMService team for config information.

### Step 2: Check Hawkeye RCA Events

Hawkeye can identify **MANA init failures**, one of the most common causes for Underhill IcMs.

```kql
let fn_nodeId = "6f05403f-aa93-9e63-2529-a747829c1aec";
let fn_containerId = "3f9f903c-24ea-4655-a7c2-a583633a88da";
let fn_faultTime = datetime(1/23/2025 11:30:15 AM);
let fn_startTime = fn_faultTime-2d;
let fn_endTime = fn_faultTime+2d;
cluster('hawkeyedataexplorer.westus2').database('HawkeyeLogs').HawkeyeRCAEvents
| where NodeId == fn_nodeId
| where Scenario has "ContainerUnresponsive" or Scenario has "ContainerStart" or Scenario has "ContainerFault"
| project NodeId, FaultTime, Scenario, RCALevel1, RCALevel2, EscalateToTeam, EscalateToOrg
```

### Step 3: Determine the Underhill Version

**Via file version (vmfirmwareigvm.dll):**
```kql
let fn_startTime = datetime(11-02-2023 07:35);
let fn_endTime = datetime(11-02-2023 21:35);
let fn_nodeId = "f30c2d3d-f286-a9e8-baa5-11fcf5e397af";
cluster('azcore.centralus').database('Fa').OsFileVersionTable
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where FileName == "vmfirmwareigvm.dll"
| where FileVersion != "FileNotFound"
| project PreciseTimeStamp, FileName, FileVersion, FileTimeStamp, FileSize
```

> **Note:** Logging occurs periodically (every day) and upon node updates. File version is unique per released version.

### Step 3a: Determine the Underhill Git Commit

```kql
let fn_startTime = datetime(01-17-2024 07:35);
let fn_endTime = datetime(01-17-2024 21:35);
let fn_nodeId = "54652100-4831-284f-7b4e-056e688fef5c";
let fn_containerId = "fcc22b1b-bdca-476e-8891-8fb9a9243b07";
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

**Given an Underhill DLL version, find the Git Commit:**
```kql
cluster('wdgeventstore.kusto.windows.net').database('CCA').GetUnderhillBinaryCommitHash('1.2.98.0')
| take 3
```

> The crate_revision corresponds to the commit hash in the **internal hvlite repo**. Check the `release/<version>` branch.

### Step 4: Compare with Known Issues

Check the Underhill Known Issues page. If no known issue matches, continue.

### Step 4a: Determine if the VM Failed to Start (VTL0 Start Failure)

If Underhill cannot make enough progress to execute the first VTL return (start executing guest code), it fires **Event 18620** (MSVM_START_VTL0_REQUEST_ERROR).

```kql
let fn_startTime = datetime(09-29-2023 07:35);
let fn_endTime = datetime(09-30-2023 21:35);
let fn_nodeId = "452f9e06-865b-b10e-5983-b61050b8e495";
let fn_containerId = "6714F5D1-4CDB-4302-9733-5F6C9A2BCEE5";
cluster('azcore.centralus').database('Fa').HyperVWorkerTable
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where EventMessage has fn_containerId
| where EventId == 18620
| extend MessageParsed = parse_json(Message)
| project TIMESTAMP, ProviderName, Level, EventId, ErrorCode = MessageParsed.ErrorCode, Status = MessageParsed.Status, ResultDocument = MessageParsed.ResultDocument, Message, EventMessage
```

**Example ResultDocument (showing NVMe failure):**
```json
{
  "error_id":"Underhill.StorageCannotOpenVtl2Device",
  "message":"cannot open nvme namespace: nvme device 44ae:00:00.0 error: failed to get namespace 1: namespace not found",
  "file_name":"underhill/underhill_core/src/dispatch/vtl2_settings_worker.rs",
  "line":675
}
```

### Error Message Triage Flowchart

| Error Contains | Category | Route To |
|---|---|---|
| `nvme` or `StorageCannotOpenVtl2Device` | Storage | Check Virtual System Identifier → ASAP → Host Storage Acceleration; NVMe Direct → zHYP SME DAS; SCSI → zHYP SME SVP |
| `mana` | Networking | "failed to start mana device" → Mana SME / Host Networking / Triage; Otherwise → zHYP SME LOW |
| `vmgs` | NVRAM | Transfer to MVM SME Team (zHYP SME MVM) |
| Other | Unknown | Search hvLite codebase |

### Storage Device Discovery Query
```kql
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

### Step 4b: Common VTL2 Failures

**Underhill Guest Crash Event (Event 18590):**
```kql
let fn_nodeId = "9d7d5e9d-ed85-533d-5e11-38165c4d50f7";
let fn_containerId = "0beaf659-47c6-4646-8ad6-54f284b4aa77";
let fn_faultTime = datetime(2025-01-18T22:37:54Z);
let fn_startTime = fn_faultTime-6h;
let fn_endTime = fn_faultTime+6h;
cluster("azcore.centralus").database("Fa").HyperVWorkerTable
| where TIMESTAMP between (fn_startTime .. fn_endTime)
| where NodeId == fn_nodeId
| where Message has fn_containerId
| where EventId == 18590
| project TIMESTAMP, Level, EventId, EventMessage = iif(isnotempty(EventMessage), EventMessage, Message)
| sort by TIMESTAMP desc
```

**Example MANA init failure panic output:**
```
[   51.991939] [U] thread 'worker-UnderhillWorker' panicked at /.../guest_emulation_transport/src/client.rs:526:25:
[   51.991948] [U] should have been terminated after reporting start failure: failed to create mana device: 
failed to initialize mana device: query_max_resources: HWC request failed. request=0x2, activity_id=0x98ab0001...
MANA request timed out. Waiting for HWC interrupt.: deadline exceeded
```

**Special cases:**
- If kmsg is empty → extract kmsg logs from Underhill dump
- If kmsg shows `SharedMemoryAllocationError (SharedPoolOutOfMemory {size: 16, tag: "igvm_attest"})` → engage **Hyper-V MVM**
- Running TDX CVM on AH2024 (not AH2024.1) is NOT supported → engage OneFleet Node/ConfidentialComputing

### Step 4c: Underhill Crash Leading to StartContainer Failure

**Crash Flow:**
1. Linux kernel invokes `/bin/underhill-crash` (configured via `core_pattern`)
2. Writes crash dump via VMBus Crash Dump Device to host
3. Host's WER service queues dump for upload to Watson
4. HyperVWorkerTable logs EventId 18590
5. After 3-4 successive failures within fault policy window (~20 min), NodeService surfaces container fault (FaultCode 10005, statusCode 0x80078000)

> **For Confidential VMs (SEV-SNP, TDX):** Crash dump is NOT sent to host. Watson dumps unavailable.

**Fleet-wide monitoring:** Theseus automation monitors `Underhill_Crashes` table in `cluster('hostosdata.centralus').database('NFP')` and auto-creates IcM incidents.

### Sample Incidents

| ICM ID | Description |
|--------|-------------|
| 587349254 | MANA init failure causes container unresponsive |
| 587349257 | MANA init failure causes container fault (FaultCode 10005) |
| 581449938 | MANA init failure causes Watson crash detection |
| 588653530 | Corrupted NVRAM state causes Watson crash |
| 589076221 | Underhill servicing fails during restore (protocol negotiation failure) |
| 772131190 | ARM64 speculative execution firmware bug causes VTL2 memory corruption (GB200/GB300) |

---

## 17. Underhill Kusto Queries & FAQ

**Source:** https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-kusto-queries-faq

### How to Identify Clusters Where Underhill is in Use

```kql
VmServiceContainerOperations 
| where PreciseTimeStamp > ago(7d) 
| where IsUnderhillLocalEnabled == true 
    or IsUnderhillNetworkEnabled == true 
    or IsUnderhillRemoteEnabled == true 
| summarize dcount(NodeId) by Cluster
```

**Alternative:**
```kql
HyperVVmConfigSnapshot 
| where PreciseTimeStamp > ago(7d) and IsUnderhill == 'true' 
| summarize dcount(NodeId) by Cluster
```

**Combined ARM Underhill Clusters:**
```kql
let underhillClusters = VmServiceContainerOperations
    | where PreciseTimeStamp > ago(7d)
    | where IsUnderhillLocalEnabled == true or IsUnderhillNetworkEnabled == true or IsUnderhillRemoteEnabled == true
    | summarize dcount(NodeId) by Cluster;
let armClusters = cluster('hostosdata.centralus.kusto.windows.net').database('HostOsData').OverlakeClusterVersions
    | extend MajorGen=toint(MajorGen)
    | where clusterType == "Compute" and isARM == true and MajorGen == 9
    | summarize count() by clusterType, Cluster, ProcessorManufacturer, OverlakeVersion, generation, Region;
underhillClusters | join kind=inner armClusters on Cluster
```

### Find a VM from Computer Name

```kql
let fn_subscriptionId = '1230ba44-d2cf-48ff-b486-46f995818c06';
let fn_roleInstanceName = "s-np-590950d1";
cluster("AzureCM").database("AzureCM").LogContainerSnapshot 
| where TIMESTAMP >= ago(7d) 
| where subscriptionId == fn_subscriptionId 
| where roleInstanceName has fn_roleInstanceName 
| distinct nodeId, containerId, virtualMachineUniqueId, subscriptionId, roleInstanceName, Tenant, tipNodeSessionId
```

**Mapping:**
- `containerId` == 'VmName' in Underhill
- `virtualMachineUniqueId` == 'VmId' (Unique ID) in Underhill
- `Tenant` == Cluster

### Where are Underhill Events in Kusto?

Underhill logs emit to **UnderhillEventTable** in the **AzureHostOs database** at **wdgeventstore.kusto.windows.net** through the **Microsoft.Windows.HyperV.Hcl** provider.

**Most useful filter fields:** VmId, VmName (container ID), TIMESTAMP, NodeId, Level, Message.

### Combined HyperV Tables Query (Underhill + VMMS + Worker + Hypervisor + VPCI)

```kql
let fn_nodeId = '94db1e7e-f598-4c19-ab60-a9f423a5e3ef';
let fn_containerId = '42390b9f-16fc-4761-8999-017175e7daf1';
let fn_startTime = datetime(2025-09-15 23:53:13) - 5m;
let fn_endTime = datetime(2025-09-15 23:53:13) + 1m;
let fn_filter = dynamic(['vmid', 'vmname', 'virtualmachineid', 'virtualmachinename', 'fields', 'level', 'timestamp', 'op_code', 'related_activity_id', 'activity_id']);

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
    | where Level <= 4
    | extend Table = "vmms";

let vmwp = cluster('azcore.centralus').database('Fa').HyperVWorkerTable
    | where NodeId == fn_nodeId and Message has fn_containerId
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime)
    | where Level <= 4
    | extend Table = "vmwp";

let vmhv = cluster('azcore.centralus').database('Fa').HyperVVHypervisorTable
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
| project PreciseTimeStamp, Table, Level, TaskName, Opcode, EventMessage = coalesce(EventMessage, Message), ActivityId, RelatedActivityId
```

### Underhill Version (from HyperVVmConfigSnapshot)

```kql
let fn_startTime = datetime(11-02-2023 07:35);
let fn_endTime = datetime(11-02-2023 21:35);
let fn_nodeId = "f30c2d3d-f286-a9e8-baa5-11fcf5e397af";
let fn_containerId = "c4c7737e0-f408-40a7-9856-cec5d2085c3a";
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot 
| where TIMESTAMP between (fn_startTime .. fn_endTime) 
| where NodeId == fn_nodeId and ContainerId == fn_containerId 
| where SummaryJson contains "vmfirmwareigvm" 
| extend m = parse_json(SummaryJson) 
| extend vtl = parse_json(m.ManagementVtlState) 
| extend state = parse_json(m.VmState) 
| project state.Current, vtl.CurrentFileName, vtl.CurrentFileVersion
```

### How Much VM Memory is Configured for VTL2?

```kql
let fn_startTime = datetime(11-02-2023 07:35);
let fn_endTime = datetime(11-02-2023 21:35);
let fn_nodeId = "f30c2d3d-f286-a9e8-baa5-11fcf5e397af";
let fn_containerId = "c4c737e0-f408-40a7-9856-cec5d2085c3a";
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot 
| where TIMESTAMP between (fn_startTime .. fn_endTime) 
| where NodeId == fn_nodeId and ContainerId == fn_containerId 
| where SummaryJson contains "Vtl2RamBaseAddrOffsetMb" 
| extend m = parse_json(SummaryJson) 
| extend mem = parse_json(m.Memory) 
| project mem.Vtl2RamBaseAddrOffsetMb, mem.Vtl2RamSizeInMb, mem.Vtl2MmioBaseAddrOffsetMb, mem.Vtl2MmioSizeInMb
```

### Periodic Memory Status (post-AH2023 GA)

Search uhdiag kmsg output for `periodic_memory_status`. Shows memory for processes: `underhill`, `underhill-init`, `underhill-vm`, `vsock_pty`.

### Check VTL2 Settings (Initial)

```kql
let fn_faultTime = datetime(2024-01-13T20:32:34Z);
let fn_nodeId = "2adbf84b-a16b-c637-6e67-cb2ffae34a12";
let fn_containerId = "ded30db5-9d14-4bf2-bba6-f002feea9b51";
cluster('azcore.centralus').database('Fa').UnderhillEventTable 
| where NodeId == fn_nodeId and VmName == fn_containerId 
| where TIMESTAMP <= fn_faultTime 
| where Message has "Initial VTL2 settings" 
| extend InternalMessage = parse_json(tostring(parse_json(Message).Message)) 
| parse InternalMessage with * "Vtl2SettingsFixed {" vtl2SettingsFixed "}" * "Vtl2SettingsDynamic {" vtl2SettingsDynamic 
| project PreciseTimeStamp, vtl2SettingsFixed, vtl2SettingsDynamic
```

### Check VTL2 Settings (Updated During Fault Window)

```kql
let fn_faultTime = datetime(2024-01-13T20:32:34Z);
let fn_startTime = fn_faultTime - 5m;
let fn_endTime = fn_faultTime + 5m;
let fn_nodeId = "2adbf84b-a16b-c637-6e67-cb2ffae34a12";
let fn_containerId = "ded30db5-9d14-4bf2-bba6-f002feea9b51";
cluster('azcore.centralus').database('Fa').UnderhillEventTable 
| where NodeId == fn_nodeId and VmName == fn_containerId 
| where TIMESTAMP between (fn_startTime .. fn_endTime) 
| where Message contains "Received VTL2 settings" 
| extend new_vtl2_settings = parse_json(tostring(parse_json(Message).Message)).fields 
| project PreciseTimeStamp, new_vtl2_settings
```

### Check SoC MANA Logs

```kql
let fn_NodeId = "5007c164-99c4-2760-6df5-762bb08ec011";
let fn_StartTime = datetime(2024-02-24);
let fn_EndTime = datetime(2024-02-25);
let socID = toscalar(cluster('azuredcm.kusto.windows.net').database('AzureDCMDb').GetSocOrNodeFromResourceId(fn_NodeId));
cluster('azcore.centralus.kusto.windows.net').database('OvlProd').LinuxOverlakeSystemd 
| where NodeId =~ fn_NodeId or NodeId =~ socID 
| where PreciseTimeStamp between (fn_StartTime .. fn_EndTime) 
| where _SYSTEMD_UNIT startswith "socmana" or _SYSTEMD_UNIT startswith "gdma-vfio" or _SYSTEMD_UNIT startswith "soc-mana-boot" 
| project LogTime = PreciseTimeStamp, _SYSTEMD_UNIT, _PID, MESSAGE 
| order by LogTime asc
```

> **SoC MANA version:** The **v** in `[bnic v=a835a9 h=0316]` corresponds to the Git Commit in the **SmartNIC-SW-GDMA Repo**.

### Check ASAP (Azure Storage Acceleration Platform) Logs

```kql
let startTime=datetime(2025-04-29T10:26:59Z);
let endTime=datetime(2025-04-29T10:46:59Z);
let nodeid="654df51e-f106-a823-e775-55ae5d2333b9";
union cluster('storageclient.eastus.kusto.windows.net').database('Fa').AsapNvmeEtwTraceLogEventView 
| where NodeId == nodeid 
| where PreciseTimeStamp between (startTime .. endTime)
```

### How Many Azure VMs Use Each HCL Version by Region?

```kql
cluster('azcore.centralus').database('Fa').HyperVVmConfigSnapshot 
| where PreciseTimeStamp > ago(4h) 
| where SummaryJson contains "vmfirmwareigvm" or SummaryJson contains "vmfirmwarecvm" or SummaryJson contains "vmfirmwarehcl" 
| summarize arg_max(PreciseTimeStamp, *) by VmId 
| extend m = parse_json(SummaryJson) 
| extend vtl = parse_json(m.ManagementVtlState) 
| extend HCLFile = tostring(vtl.CurrentFileName), HCLVersion = tostring(vtl.CurrentFileVersion) 
| project Region, VmId, HCLFile, HCLVersion
| summarize dcount(VmId) by HCLFile, HCLVersion, Region
```

### How to Collect a Dump of the VM's Underhill Environment

**Manually:** Set registry key to save VM on triple fault:
```
HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Virtualization\Worker\SaveVmOnTripleFaultFile
```

```powershell
reg add "HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Virtualization\Worker" /v SaveVmOnTripleFaultFile /t REG_SZ /d "c:\resources\virtual machines\dump.vmrs" /f
```

### How to Extract kmsg from an Underhill Dump

1. Obtain `uhext.min.js` (Underhill WinDbg extension) from Underhill pipeline build
2. Open in WinDbg (File → Open Script)
3. Press Ctrl-S (green checkmark = loaded)
4. Execute `!kmsg`

### How to Inject NMI into VM's VTL2

Use `vmadmin.js` to inject NMIs into VTL2 using Hyper-V's `InjectNonMaskableInterruptEx` WMI API. Useful when guest is stuck.

### Tools: uhdiag

`uhdiag` / `uhdiag-dev.exe` is present on Azure VHD by default. Used for:
- Inspecting VM state (`uhdiag-dev.exe <containerId> inspect vm`)
- Getting kmsg logs (`uhdiag-dev.exe <containerId> kmsg`)
- Network diagnostics (`uhdiag.exe network-diagnostic-data v1 <ID>`)
- Packet capture (`uhdiag-dev.exe <vmname> packet-capture -G 60 d:\pcap\uh-nw`)
- Unsticking vmbus channels (`uhdiag-dev.exe <containerId> inspect vm/vmbus/unstick_channels -u true`)
- Setting trace filters (`D:\vmadmin\vmadmin.cmd <containerId> inspect trace/filter -u info,netvsp=debug,vmbus=debug`)

---

## 18. Underhill Out of Memory (OOM)

**Source:** https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-oom

Underhill is effectively an embedded environment with tight memory constraints.

### Verify OOM Occurred

```kql
let fn_faultTime = datetime("2024-03-08 22:49:00");
cluster('azcore.centralus.kusto.windows.net').database('SharedWorkspace').UnderhillVtl2OOM(
    _startTime=datetime_add('day', -1, fn_faultTime),
    _endTime=datetime_add('day', 1, fn_faultTime),
    _containerIds=dynamic(['34a3dd71-1e11-4052-ab46-35cba250c85a']),
    _nodeIds=dynamic(['a33a2643-f5cf-3799-476b-fafd9a37fbcb'])
)
```

### Key Fields from OOM Query
1. **CurrentFileVersion** - Check against known issues
2. **VmSize** (VM SKU) - Look up VPs, local storage, NIC count
3. **Message/EventMessage** - OOM details
4. **Vtl2RamSizeInMb** - Memory allocated to Underhill

### Determine Kernel vs User-Space OOM

1. **Extract AnonPages** from OOM message (`active_anon + inactive_anon`)
2. **Get idle AnonPages baseline** using:
```kql
let fn_firmwareVersion="1.2.76.0";
let fn_vmSize="Standard_E2bds_v5";
cluster('azcore.centralus.kusto.windows.net').database('SharedWorkspace').UnderhillMemorySnapshotsV1(
    _startTime=startofmonth(datetime(now),-1), 
    _endTime=endofmonth(datetime(now),-1)
) 
| where Process == "Kernel" and Metric == "AnonPages_KiB" 
| where FirmwareVersion == fn_firmwareVersion and VmSize == fn_vmSize
| summarize percentiles(Value, 0, 1, 10) by TIMESTAMP=startofmonth(TIMESTAMP), VmSize, FirmwareVersion
```

3. **Add storage overhead** (by VP count):

| VPs | RemoteStorage(MiB) | LocalStorage(MiB) |
|-----|-------------------|-------------------|
| 2 | 0.40 | 0.33 |
| 4 | 0.93 | 0.74 |
| 8 | 2.5 | 2.6 |
| 16 | 9.4 | 10.4 |
| 32 | 35.3 | 64.4 |
| 48 | 53 | 89.5 |
| 64 | 69 | 116.5 |
| 96 | 106.7 | 171.7 |

4. **Add networking overhead:** `NetworkingOverhead(KiB) = min(8, VPs) * NIC_Count * 315.5`
5. **MaxAnonPages = Idle + StorageOverhead + NetworkingOverhead**
6. If OOM AnonPages > MaxAnonPages by 20% or ~10 MiB → **User-space issue**; otherwise → **Kernel issue**

### ICM Routing for OOM

| Component | Route To |
|-----------|----------|
| Kernel | LSG\Triage |
| underhill-profi (profiler) | Azure Profiler\Incident Manager |
| Other User-Process | RDOS |
| Unknown/Ambiguous | RDOS |

---

## 19. Underhill Servicing

**Source:** https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-servicing

### Overview
Underhill servicing updates the Underhill version running in VMs. VMs pick up the version in `vmfirmwareigvm.dll` on cold-start or reset. Servicing updates Running VMs with minimal guest-visible impact (generally sub-second glitch).

### Policy Settings

**Per-VM:** `ManagementVtlUpdatePolicy` in VM VSSD
- Value 0 / "Default" = No restriction
- Value 1 / "OfflineOnly" = Cannot be serviced (e.g., CVMs)

**Global:** `HKLM\Software\Microsoft\Windows NT\CurrentVersion\Virtualization\ManagementVtlUpgradePolicy`
- Value 0 = No restriction
- Value > 0 = Servicing disabled for all VMs on node

### Servicing Mechanism (8 Steps)
1. **VMMS Validation** → checks Underhill VM, version, Running state
2. **Worker Process Validation** → re-validates, checks policies
3. **IGVM File Loading** → loads from vmfirmwareigvm.dll, compares versions
4. **Management VTL Save State** → ⚠️ **Point of no return** - failure after this requires VM reset
5. **VP and VTL Preparation** → stops VPs, disables VTL protections
6. **Load New VTL Environment** → from IGVM file
7. **Resume VPs & First Boot** → new environment starts
8. **Saved State Restoration** → sends saved state, reports success

### Key Servicing Event IDs
| Event ID | Meaning |
|----------|---------|
| 5124 | Failed servicing operation |
| 5126 | Successful servicing operation |
| 5128 | VM reset due to servicing failure |
| 5136 | Servicing failed due to guest power event |

### Check for Servicing Attempts During Impact Window

```kql
let fn_impactTime = datetime("2024-05-07T16:35:19Z");
let fn_startTime = fn_impactTime - 10m;
let fn_endTime = fn_impactTime + 10m;
let fn_nodeId = "3294bd07-9f08-501d-d62c-c2bcea0cb027";
let fn_vmId = "cdc43451-f401-4a64-9795-ef5b2eb239a1";
cluster('azcore.centralus').database('Fa').HyperVVmmsTable 
| where TIMESTAMP between (fn_startTime .. fn_endTime) 
| where NodeId == fn_nodeId and Message has fn_vmId 
| where TaskName == "VmmsAutomaticManagementVtlReloadDispatch" or TaskName == "ReloadManagementVtlVmmsTaskDispatch" 
| project TIMESTAMP, TaskName, Opcode, Message, ActivityId, RelatedActivityId
```

### Servicing Failure Error Codes

| Error Code | Description |
|---|---|
| 0xC0370800 | Servicing already in progress |
| 0xC0370801 | Invalid protocol response from management VTL |
| 0xC0370802 | Management VTL failed to save state (resumes with no guest impact) |
| 0xC0370803 | Management VTL failed to restore (often saved state corruption) |
| 0xC037080a | Failed to establish GET protocol with management VTL (timeout) |

### Servicing Stages (in 5124 failures)
None → ReadNewIgvm → FindHostDevicesToReset → SaveManagementVtlState → StopVps → DisableManagementVtl → ResetHostState → LoadNewIgvmFile → RestoreManagementVtlState

### Check for Stuck Servicing Operations

```kql
-- Finds every failed patch that logged entering servicing_save_vtl2 but did not log an exit
let startTime = datetime(2024-02-14);
let endTime = now();
let UHFailures = materialize(
    cluster('wdgeventstore.kusto.windows.net').database('CCA').UnderhillServicingExecutionData
    | where UnderhillSvcPackageExecutionStartTimeStamp between (startTime .. endTime)
    | where EventId == long(5124)
    | where UnderhillSvcPkgStatus == "PATCH_FAILED_CANCELED"
    | where Source startswith "UnderhillSvc"
    | project RunId, CorrelationId, NodeId, VmName, UnderhillSvcPkgStatus, ServicingStage, OldVmFirmwareIgvmVersion);
-- ... (see full query in TSG)
```

### Servicing Blackout Time (Guest Glitch Duration)

```kql
cluster('wdgeventstore.kusto.windows.net').database('AzureHostOs').UnderhillEventTable 
| where TIMESTAMP between (fn_startTime .. fn_endTime) 
| where NodeId == fn_nodeId and Message has fn_correlationId 
| where Message has "blackout"
```

**Three phases:** (1) Underhill save, (2) Linux kernel boot, (3) Underhill restore. SLA is generally sub-second.

### Manual Servicing Invocation

```powershell
$Vm = Get-Vm "<your VM name>"
$guestManagementService = Get-CimInstance -Namespace "root\virtualization\v2" -ClassName "Msvm_VirtualSystemGuestManagementService"
$guestManagementService | Invoke-CimMethod -name "ReloadManagementVtl" -Arguments @{
    "VmId" = $Vm.Id.ToString(); "Options" = 0; "TimeoutHintSecs" = 120
}
```

**CLI:** `uhservicing.wsf upgrade` (preferred) or `vmadmin.cmd ReloadVtl2`

---

## 20. Underhill Networking

**Source:** https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-networking

### Check if Underhill is Configured for Networking

```kql
cluster('wdgeventstore.kusto.windows.net').database('AzureHostOs').UnderhillEventTable 
| where NodeId == fn_nodeId and Message has "nic_devices" 
| parse Message with * "nic_devices: [" nicDevices "]" * 
| extend UnderhillIsConfiguredForNetworking = iff(isnotempty(nicDevices), true, false) 
| project PreciseTimeStamp, UnderhillIsConfiguredForNetworking, nicDevices
```

### Synthetic vs Accelerated Networking

Three configurations:
1. Not configured for networking
2. Synthetic only (instance_id present, subordinate_instance_id = None)
3. Both Synthetic + Accelerated (subordinate_instance_id = Some(...))

### Is NIC Running in Synthetic or Accelerated Mode?

Check `direction_to_vtl0`:
- **1** = Accelerated Networking mode
- **0** = Synthetic Networking mode

### MANA Logs Query

```kql
let fn_startTime = datetime(9/10/2025);
let fn_endTime = datetime(9/11/2025);
let fn_nodeId = '694cce83-6d07-9f69-1058-8d374b1ffc89';  

let iov = cluster('netperf').database('NetPerfKustoDB').ManaVNicIovEvents()
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime) and NodeId == fn_nodeId
    | extend Table = 'ManaVNicIov';
let umed = cluster('netperf').database('NetPerfKustoDB').ManaUmedEvents()
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime) and NodeId == fn_nodeId
    | where TaskName has 'MANA' | extend Table = 'ManaUmed';
let im = cluster('netperf').database('NetPerfKustoDB').ManaImEvents()
    | where PreciseTimeStamp between (fn_startTime .. fn_endTime) and NodeId == fn_nodeId
    | extend Table = 'ManaIm';
union iov, umed, im | project PreciseTimeStamp, Table, EventMessage, Message, TaskName
```

### Packet Capture via Underhill

```powershell
mkdir d:\pcap
D:\vmadmin\vmadmin.cmd list  # Pick a VM name
uhdiag-dev.exe <vmname> packet-capture -G 60 d:\pcap\uh-nw
# Generates uh-nw-*.pcap files (one per vNIC)
```

Options: `-s 128` for packet length (just TCP/IP headers), `-G <seconds>` for duration.

### Network Diagnostics

```powershell
uhdiag.exe network-diagnostic-data v1 <containerID>
```

Returns large JSON blob with per-NIC stats (in/out packets, octets, queue info, ring size optimization status).

---

## 21. Underhill Specific Incident: ICM 676743226

**Source:** https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/underhill/underhill-incident-676743226

### Problem
Guest OSs with ring size optimization issues cause missing interrupts. Setting `ring_size_limit` to 0 forces poll mode (performance impact but prevents stuck traffic).

### Mitigation

```powershell
# Inspect the VM's network device
$n | Invoke-IAgentInvokeCommand "uhdiag-dev.exe $container_id inspect vm/net:f8615163-0000-1000-2000-<mac>"

# Disable ring size optimization
$n | Invoke-IAgentInvokeCommand "uhdiag-dev.exe $container_id inspect vm/net:f8615163-0000-1000-2000-<mac>/ring_size_limit -u 0"
```

### Unstick VMBus Channels

```powershell
$n | Invoke-IAgentInvokeCommand "uhdiag-dev.exe $container_id inspect vm/vmbus/unstick_channels -u true"
```

### Collect VmBus Counters for Diagnostics

```powershell
$n | Invoke-IAgentInvokeCommand "uhdiag-dev.exe $container_id inspect vm/vmbus -r" > C:\temp\$container_id.vmbus.inspect.$num.txt
```

---

## 22. VMGS TSG

**Source:** https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/tsg-vmgs

### VMGS Overview
VMGS (Virtual Machine Guest State) contains TPM state, UEFI BIOS NVRAM variables, and other boot-persistent data.

### VM Type VMGS Formats
- **Trusted Launch V1 / Confidential VMs:** VMGSv3, on shared storage with OS disk
- **Non-TL Underhill VMs:** VMGSv3, on local node storage (lost on deallocation)
- **Gen 1/2 without HCL:** VMGSv1, on local node storage

### VmgsTool Exit Codes
| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Not encrypted |
| 3 | File empty (expected during initial deployment) |
| 4 | File ID not found |
| 5 | V1 format when V3 expected |
| 6 | Encrypted using GspById method |

### Known VMGS Issues
1. VMGSv1 from TL VM on RS1.85
2. UEFI boot failure after boot disk change
3. Windows upgrade failure
4. VmgsTool timeout (can corrupt VMGS)
5. No space for new encryption key
6. VMGS file offline
7. Unknown encryption scheme

### Mitigating Corrupted VMGS
1. Download OS Disk to storage account
2. Create new Managed Disk without VM guest state blob
3. Swap OS disk
4. Fresh VMGS provisioned on start

**Consequences:** Bitlocker recovery needed, custom boot entries lost, TPM secrets need reconfiguring.

---

## 23. Watchdog Timeouts

**Source:** https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/uefi/uefi-watchdog-timeout

### What is a Watchdog Timeout?
- UEFI Boot Services has a **2-minute timeout**
- If exceeded: host triggers triple-fault (halt or NMI)
- Automatic reboot cycle; host attempts several restarts before terminating VM

### For Standard Gen2 / Legacy HCL VMs
Search HyperVWorkerTable for Event 18600 ("has encountered a watchdog timeout").

### For OpenHCL/Underhill VMs
Search UnderhillEventTable for "Encountered a watchdog timeout".

```kql
cluster('azcore.centralus').database('Fa').UnderhillEventTable      
| where NodeId in (fn_nodeId) and VmName == fn_containerId     
| where Message has_any ("Encountered a watchdog timeout")     
| project PreciseTimeStamp, NodeId, VmName, VmId, Level, Message
```

Then look back 2 minutes to see UEFI activity before timeout.

---

## 24. EfiDiagnostics

**Source:** https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/uefi/uefi-efidiagnostics

### Overview
Available for VMs on **AH2025+**. NOT available for hibernate-compatibility mode.

### Log Routing
- **OpenHCL/Underhill VMs:** Logs route to kmsg and **UnderhillEventTable**
- **Standard Gen2 / Legacy HCL VMs:** Logs route to **HyperVEfiDiagnosticsTable**

### Default: ERROR and WARN levels only

### Query EfiDiagnostics for Underhill VMs

```kql
cluster('azcore.centralus').database('Fa').UnderhillEventTable      
| where NodeId in (fn_nodeId) and VmName == fn_containerId     
| extend RawFields = parse_json(Message).Fields     
| extend ParsedFields = parse_json(tostring(RawFields))     
| extend UefiMsg = tostring(ParsedFields.log_message)     
| extend UefiLevel = tostring(ParsedFields.debug_level)     
| extend UefiPhase = tostring(ParsedFields.phase)     
| extend UefiTicks = tostring(ParsedFields.ticks)     
| project PreciseTimeStamp, NodeId, VmName, VmId, Level, UefiLevel, UefiPhase, UefiMsg, UefiTicks
```

### Known Red Herrings (False Positives)
1. "THIS BOOT MODE IS UNSUPPORTED. 0x0" - Not a real error
2. "[Bds] Unable to boot!" - Normal on first boot for Underhill/Legacy HCL VMs
3. "Error: Image at ... start failed: Unsupported" - Upstream DXE logic, not real issue
4. "ConvertPages: ..." - Page alignment noise, not real issue

### Quick UEFI Elimination
If you can prove VM entered bootmgr or left ExitBootServices, UEFI is NOT the issue.

---

## 25. Incident Tagging and Routing

**Source:** https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/incident-tagging

### Tag System
Tags named: `[TeamName]-[KnownIssue]` (e.g., `HYP-OOM`, `HYP-Crash`)

### Key Tags
| Tag | Description |
|-----|-------------|
| HYP-Auto-Misroute | Misrouted by software |
| HYP-Manual-Misroute | Misrouted by person |
| HYP-VMGSCorrupt | VMGS corruption |
| HYP-OOM | VM start failure - out of memory |
| HYP-StopStuckInNetwork | Stop container stuck in network stack |
| HYP-Crash | Hyper-V application crashed |
| HYP-ResourceLeak | VM leaking handles/memory |
| HYP-LKDAvailable | Live kernel dump available |
| HYP-FixPendingDeployment | Fix identified, pending fleet deployment |

### Team Ownership for Underhill

| Team | Owns | Lead |
|------|------|------|
| **Storage Virtualization Platform (SVP)** | "General Contractor" for Underhill; SCSI→NVMe translation | svpdev@microsoft.com |
| **Modern Virtualization (MV)** | Underhill Servicing, MHP servicing | modernvirt-dev@microsoft.com |
| **Modern VM (MVM)** | Firmware (PCAT gen1, UEFI gen2), vTPM, VMGS | hypmvm@microsoft.com |
| **Linux on Windows (LOW)** | VmBus, HvSocket, open source virtualization | lowdev@microsoft.com |
| **Devices and Storage (DAS)** | Device virtualization (vPCI, NVMe Direct, GPU) | das-dev@service.microsoft.com |
| **Hypervisor Core (HC)** | Hypervisor memory, scheduling, CVM, VTLs | hvcoredev@microsoft.com |

---

## 26. Known Error Messages to Ignore

These are from the Linux kernel running in an unfamiliar environment and are NOT indicative of real problems:

| Target | Message |
|--------|---------|
| kmsg | "Error: No information about IO-APIC in OF." |
| kmsg | "Cannot find an available gap in the 32-bit address range" |
| kmsg | "PCI devices with unassigned 32-bit BARs may not work!" |
| kmsg | "RETBleed: WARNING: Spectre v2 mitigation leaves CPU vulnerable to RETBleed attacks, data leaks possible!" |
| kmsg | "PCI: Fatal: No config space access function found" |

---

## 27. Key Contacts and Escalation

| Role | Contact |
|------|---------|
| **Hyper-V SME (general)** | hypsme |
| **Host OS Livesite V-team** | HostOsLiveCore@microsoft.com |
| **RDOS DRI** | IcM queue: RDOS / Azure Host OS DRI Sev 1-2 or Sev 3-4 |
| **Hawkeye** | teamelixir@microsoft.com |
| **Gandalf** | Ze Li (zeli), Vivek Ramamurthy (viramam) |
| **VMA** | Ze Li (zeli), Nick Swanson (nicksw) |
| **RDOS RA Tool** | hostosarbot@microsoft.com |
| **Host OS Servicing** | HostUpdate@microsoft.com |
| **HCM Triage** | HCMTriage@microsoft.com |
| **Documentation** | HostOsLiveDocs@microsoft.com |

---

## 28. Quick Reference: Essential Queries Cheat Sheet

| What | Query/Table | Cluster/DB |
|------|-------------|------------|
| Is VM Underhill? | `HyperVVmConfigSnapshot` → `IsUnderhill` | azcore.centralus / Fa |
| Underhill events | `UnderhillEventTable` | wdgeventstore / AzureHostOs |
| Underhill version | `OsFileVersionTable` → `vmfirmwareigvm.dll` | azcore.centralus / Fa |
| Underhill git commit | `GetUnderhillBinaryCommitHash()` | wdgeventstore / CCA |
| VM start failure (VTL0) | `HyperVWorkerTable` → EventId 18620 | azcore.centralus / Fa |
| Guest crash/panic | `HyperVWorkerTable` → EventId 18590 | azcore.centralus / Fa |
| Watchdog timeout | `HyperVWorkerTable` → EventId 18600 | azcore.centralus / Fa |
| Hawkeye RCA | `HawkeyeRCAEvents` | hawkeyedataexplorer.westus2 / HawkeyeLogs |
| Servicing failures | `UnderhillServicingExecutionData` | wdgeventstore / CCA |
| OOM investigation | `UnderhillVtl2OOM()` | azcore.centralus / SharedWorkspace |
| Memory snapshots | `UnderhillMemorySnapshotsV1()` | azcore.centralus / SharedWorkspace |
| Container health | `MycroftContainerHealthSnapshot` | azcore.centralus / AzureCP |
| Host OS updates | `AnyHostUpdateOnNode()` | wdgeventstore / HostOSDeploy |
| SoC MANA logs | `LinuxOverlakeSystemd` | azcore.centralus / OvlProd |
| MANA traces | `ManaVNicIovEvents()`, `ManaUmedEvents()`, `ManaImEvents()` | netperf / NetPerfKustoDB |
| ASAP logs | `AsapNvmeEtwTraceLogEventView` | storageclient.eastus / Fa |
| Underhill crashes (fleet) | `Underhill_Crashes` | hostosdata.centralus / NFP |
| EfiDiagnostics (Underhill) | `UnderhillEventTable` → ParsedFields.log_message | azcore.centralus / Fa |
| EfiDiagnostics (non-UH) | `HyperVEfiDiagnosticsTable` | sddce2edata.westus / BaseAzHostData |
| SEL logs | `RdosSelByNodeId()` | baseplatform.westus / vmphu |

---

*Document complete. Generated from RDOS Livesite EngHub documentation traversal on 2026-04-03.*

