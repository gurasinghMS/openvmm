# HyperVStorageStackTable

**Type:** Table  
**Cluster:** `https://azcore.centralus.kusto.windows.net`  
**Database:** `Fa`  
**Full Path:** `azcore.centralus.kusto.windows.net` → `Fa` → `HyperVStorageStackTable`

---

## 1. Description

The HyperVStorageStackTable contains a **filtered view of events and traces from Hyper-V's Storage Virtualization providers**. It collects events from multiple storage-related ETW providers into a single table with the event payload stored as a JSON blob in the `Message` column. This design keeps the schema short despite the broad number of providers.

**Key Characteristics:**
- **Data Source:** ETW events from Hyper-V storage virtualization components — VHD/VHDX management, NVMe Direct, StorVSP, synthetic storage, emulated storage, and virtual PMEM
- **Provider Configuration:** Defined in `MdsFa.xml`
- **Volume:** Extremely high-throughput (~34.8 billion events per day across all regions)
- **Retention:** Typically ~60 days of historical data
- **Severity Coverage:** Captures all log levels — Error (2) ~42%, Informational (4) ~55%, Warning (3) ~3%, Verbose (5) <0.1%. No Level 1 (Critical) observed.
- **Primary Use Case:** Investigating storage I/O failures, NVMe Direct device assignment issues, VHD/VHDX open/close/resize errors, VMGS (Virtual Machine Guest State) problems, and storage performance (latency histograms)
- **Maintainer:** hypsme · IcM queue: `RDOS/Azure Host OS SME - Virtualization (Hyper-V)`

**Cluster / Database Location:**
```
cluster('azcore.centralus.kusto.windows.net').database('Fa')
```
Shorthand: `cluster('azcore.centralus').database('Fa')`

**Provider Names (by volume):**

| Provider | Volume (1d) | Description |
|----------|-------------|-------------|
| `Microsoft.Windows.HyperV.Storage.NvmeDirect` | ~10.8B | Primary NVMe Direct storage provider |
| `Microsoft.Windows.HyperV.VhdmpTrace` | ~10.6B | VHD/VHDX miniport driver trace events |
| `Microsoft.Windows.HyperV.Storage.NvmeDirect2` | ~8.5B | NVMe Direct v2 storage provider |
| `Microsoft-Windows-Hyper-V-StorageVSP` | ~2.2B | Virtual Storage Provider (StorVSP) events |
| `Microsoft.Windows.HyperV.StorageActivity` | ~1.9B | Storage activity tracing (failures, I/O perf, VMGS) |
| `Microsoft-Windows-VIRTDISK` | ~257M | Virtual disk (VHD/VHDX) operational events |
| `Microsoft-Windows-Hyper-V-SynthStor` | ~257M | Synthetic storage controller events |
| `Microsoft.Windows.HyperV.Storage.NvmeDirect2.Activity` | ~191M | NVMe Direct v2 activity tracing |
| `Microsoft.Windows.HyperV.NvmeDirect.Telemetry` | ~68M | NVMe Direct telemetry (operating mode switches) |
| `Microsoft-Windows-VHDMP` | ~36M | VHD miniport driver operational events |
| `Microsoft-Windows-Hyper-V-NvmeDirectDriver` | ~7.7M | NVMe Direct kernel driver events |
| `Microsoft.Windows.HyperV.Storage` | ~1.1M | General Hyper-V storage (WIL failures) |
| `Microsoft-Windows-Hyper-V-EmulatedStor` | ~496K | Emulated (IDE) storage controller events |
| `Microsoft-Windows-Hyper-V-Virtual-PMEM` | ~41 | Virtual persistent memory events (very rare) |

**Message Format:**
The `Message` field contains **JSON payloads** whose schema varies by `ProviderName` and `TaskName`. Always use `parse_json(Message)` and inspect field names. The `EventMessage` field contains formatted human-readable text primarily for `Microsoft-Windows-Hyper-V-SynthStor` events (e.g., EventId 12148 "started successfully").

---

## 2. Schema

| Column Name | Type | Description |
|-------------|------|-------------|
| `TIMESTAMP` | datetime | Event ingestion timestamp in UTC. **Do not use for queries** — use `PreciseTimeStamp` instead |
| `PreciseTimeStamp` | datetime | **Precise event timestamp in UTC.** Use for ALL time-based queries and retention boundaries |
| `Environment` | string | Azure environment identifier (e.g., `"PROD"`) |
| `Region` | string | Azure region (e.g., `"centralus"`, `"eastus2"`) |
| `DataCenter` | string | Physical datacenter identifier (e.g., `"DSM40"`) |
| `Cluster` | string | Azure cluster name (e.g., `"DSM40PrdApp05"`) |
| `NodeIdentity` | string | IP address of the physical compute node |
| `NodeId` | string | **Unique GUID for the physical compute node.** Primary infrastructure filter |
| `DeviceId` | string | Device identifier for the host component generating the event |
| `Level` | long | **ETW severity level:** 1=Critical, 2=Error, 3=Warning, 4=Informational, 5=Verbose |
| `ProviderGuid` | string | ETW provider GUID |
| `ProviderName` | string | ETW provider name (see Provider Names table above) |
| `EventId` | long | Numeric event identifier. `0` is the most common (trace events). Key non-zero IDs: `1012`, `1011` (VIRTDISK), `12148` (SynthStor), `302`/`303` (VHDMP) |
| `Pid` | long | Process ID |
| `Tid` | long | Thread ID |
| `OpcodeName` | string | Operation code name for the event |
| `KeywordName` | string | ETW keyword name |
| `TaskName` | string | **Task name associated with the event.** Critical for filtering — see Key Task Names section |
| `ChannelName` | string | ETW channel name |
| `EventMessage` | string | Formatted event message string (primarily used by SynthStor EventId 12148) |
| `ActivityId` | string | Activity correlation GUID |
| `Task` | long | Numeric task identifier |
| `Opcode` | long | Numeric operation code |
| `RelatedActivityId` | string | Related activity GUID for correlation |
| `Message` | string | **JSON blob containing event payload.** Primary data field — parse with `parse_json(Message)` |
| `SourceNamespace` | string | Telemetry source namespace (typically `"Fa"`) |
| `SourceMoniker` | string | Telemetry source moniker (e.g., `"FaDiagdm"`) |
| `SourceVersion` | string | Telemetry source version |
| `PartA_PrivTags` | long | Privacy tags |
| `Function` | string | Source function name (ETW-level) |
| `File` | string | Source file path (ETW-level) |
| `Line` | long | Source line number (ETW-level) |
| `Ctx` | string | Context string |
| `hresult` | long | **HRESULT error code** — populated for WIL (Windows Implementation Library) failure events from `Microsoft.Windows.HyperV.Storage` provider |
| `fileName` | string | Source file name for WIL failures (e.g., `"onecore\\vm\\dv\\storage\\scsi\\vdev\\dll\\synthstorattachment.cpp"`) |
| `lineNumber` | long | Source line number for WIL failures |
| `module` | string | Module name for WIL failures (e.g., `"vmsynthstor.dll"`) |
| `failureType` | long | WIL failure type (0=expected, 2=logged exception) |
| `message` | string | WIL failure message (lowercase — distinct from `Message`) |
| `threadId` | long | Thread ID for WIL failures |
| `callContext` | string | Call context for WIL failures |
| `originatingContextId` | long | Originating context ID |
| `originatingContextName` | string | Originating context name |
| `originatingContextMessage` | string | Originating context message |
| `currentContextId` | long | Current context ID |
| `currentContextName` | string | Current context name |
| `currentContextMessage` | string | Current context message |
| `failureId` | long | WIL failure ID |
| `failureCount` | long | WIL failure count |
| `function` | string | Function name for WIL failures (lowercase — distinct from `Function`) |
| `AutopilotEnvironment` | string | Autopilot environment string |
| `ObfuscatedData` | string | Obfuscated data field |
| `vfLuid` | long | **NVMe Direct VF (Virtual Function) LUID.** Populated for NVMe Direct telemetry events |
| `serialNumber` | string | **NVMe device serial number.** Populated for NVMe Direct telemetry events |
| `tracePeriodStartTime` | datetime | Start of the NVMe Direct trace period |
| `currentOperatingMode` | long | NVMe Direct current operating mode |
| `currentModeDurationMs` | long | Duration in current operating mode (ms) |
| `slowEntryCount` | long | NVMe Direct slow path entry count |
| `slowTotalDurationMs` | long | NVMe Direct slow path total duration (ms) |
| `slowMinDurationMs` | long | NVMe Direct slow path minimum duration (ms) |
| `slowMaxDurationMs` | long | NVMe Direct slow path maximum duration (ms) |
| `slowDurationStdDevMs` | real | NVMe Direct slow path duration standard deviation (ms) |
| `fastEntryCount` | long | NVMe Direct fast path entry count |
| `fastTotalDurationMs` | long | NVMe Direct fast path total duration (ms) |
| `fastMinDurationMs` | long | NVMe Direct fast path minimum duration (ms) |
| `fastMaxDurationMs` | long | NVMe Direct fast path maximum duration (ms) |
| `fastDurationStdDevMs` | real | NVMe Direct fast path duration standard deviation (ms) |

---

## 3. Critical Column Guide — What to Query By

### Tier 1: Always Include
| Column | Why |
|--------|-----|
| `PreciseTimeStamp` | Time-range filter is **mandatory** on this very high-volume table (~35B rows/day). Always use `PreciseTimeStamp between(start..end)` |
| `NodeId` | Scopes to a specific physical host. Required for all node-level investigations |

### Tier 2: Strongly Recommended
| Column | Why |
|--------|-----|
| `ProviderName` | Narrows to the relevant storage subsystem. Critical given 15 different providers |
| `Level` | Filter severity: `Level < 3` for errors+critical, `Level == 3` for warnings |
| `TaskName` | Identifies the type of event (e.g., `"ActivityFailure"`, `"IoPerformance"`, `"VhdopTrace"`) |
| `Message` | Use `has` or `contains` to search within JSON payload for container IDs, VM IDs, file paths, NTSTATUS codes |

### Tier 3: Post-Filter
| Column | Why |
|--------|-----|
| `EventId` | Useful for SynthStor events (12148) and VIRTDISK events (1012, 1011) |
| `DeviceId` | Scope to a specific device |
| `Pid` / `Tid` | Scope to a specific process/thread |
| `ActivityId` | Correlate related events within an operation |
| `hresult` | Filter for specific HRESULT error codes (WIL failures only) |
| `vfLuid` / `serialNumber` | NVMe Direct device-specific filtering |
| `EventMessage` | Useful for SynthStor human-readable messages |

---

## 4. Key Task Names Reference

| TaskName | Volume (1d) | Description |
|----------|-------------|-------------|
| `Trace` | ~19.3B | Generic trace events (NvmeDirect, NvmeDirect2, StorageVSP) |
| `VhdopTrace` | ~6.7B | VHD operation trace events (open, close, read, write) |
| `VhdmpTrace` | ~3.7B | VHD miniport trace events |
| _(empty)_ | ~2.5B | Events with no task name set |
| `ActivityFailure` | ~1.7B | Storage operation failures with NTSTATUS codes in Message JSON |
| `IoPerformance` | ~225M | I/O latency histogram data (latency buckets, byte counts, duration) |
| `Attach virtual disk.` | ~131M | VHD/VHDX attach operations |
| `Detach virtual disk.` | ~126M | VHD/VHDX detach operations |
| `VmgsWarning` | ~113M | VMGS file warnings (e.g., failed to get file information) |
| `FallbackError` | ~63M | Fallback error events |
| `Vhdop.VhdmpiMainIoCompletion` | ~62M | VHD main I/O completion events |
| `FioInitialize` / `FioTeardown` | ~17M each | NVMe Direct FIO (Fast I/O) lifecycle events |
| `FioNotifyHandleOpened` / `FioNotifyHandleClosing` | ~17M each | NVMe Direct FIO handle events |
| `FioAddHardwareToDevice` / `FioStartEx` / `FioStopEx` | ~17M each | NVMe Direct FIO device events |
| `OperatingModeSwitch` | — | NVMe Direct operating mode switches (in Telemetry provider) |

---

## 5. Key Event IDs Reference

| EventId | Volume (1d) | Provider(s) | Description |
|---------|-------------|-------------|-------------|
| `0` | ~21.2B | Most providers | Generic trace events — payload is in the `Message` JSON |
| _(null)_ | ~10.8B | Various | Events with no EventId set (primarily NVMe Direct) |
| `1012` | ~1.7B | `Microsoft-Windows-VIRTDISK` | Virtual disk operational event |
| `9` | ~443M | `Microsoft-Windows-Hyper-V-StorageVSP` | StorVSP event |
| `12148` | ~255M | `Microsoft-Windows-Hyper-V-SynthStor` | Synthetic storage device started successfully. Uses `EventMessage` field (human-readable) |
| `1011` | ~76M | `Microsoft-Windows-VIRTDISK` | Virtual disk operational event |
| `302` | ~69M | `Microsoft-Windows-VHDMP` | VHD miniport event |
| `3` / `4` / `5` / `6` | ~63M each | `Microsoft-Windows-Hyper-V-StorageVSP` | StorVSP lifecycle events |
| `8` | ~10M | `Microsoft-Windows-Hyper-V-StorageVSP` | StorVSP event |
| `303` | ~1.7M | `Microsoft-Windows-VHDMP` | VHD miniport event |
| `5014` / `5015` / `5016` | ~1.7M each | `Microsoft-Windows-Hyper-V-NvmeDirectDriver` | NVMe Direct kernel driver events |

---

## 6. Common Message Patterns

### VHD Operation Traces (VhdmpTrace / VhdopTrace)
Message is a JSON blob with `Source`, `Line`, and `Message` fields:
```json
{"Source":"VhdmpiOpenAutoAttachRegistryKey","Line":4507,"Message":"Return with status 0xC0000034"}
```
```json
{"Source":"VhdmpiUpdateResiliencyIoTimeout","Line":8392,"Message":" (file 'C:\\Resources\\Virtual Machines\\...vmgs')Failed to set Resiliency IO timeout..."}
```
```json
{"Source":"VhdmpiOpenBackingFileWithOptions","Line":3307,"Message":" (file 'A:\\...vhd')Failed to open backing file: 0xc0000043"}
```

### Activity Failures (StorageActivity)
Message contains NTSTATUS as a decimal number:
```json
{"Status":3221225488}
```
Convert to hex for lookup: 3221225488 = `0xC0000010` (STATUS_INVALID_DEVICE_STATE)

### I/O Performance Histograms (IoPerformance)
Large JSON with latency bucket arrays:
```json
{"PartA_PrivTags":16777216,"vhdFileName":"...","ioType":1,"collectionId":...,"intervalDurationMilliseconds":300001,"highLatencyIoCount":0,"latencyLevelsNanoseconds_Count":16,...,"ioCountArray_Count":16,...}
```
- `ioType`: 1=Read, 2=Write
- `latencyLevelsNanoseconds_*`: Latency bucket boundaries (128μs to max)
- `ioCountArray_*`: Count of IOs in each latency bucket
- `totalDurationNanosecondsArray_*`: Total duration per bucket
- `totalBytesArray_*`: Total bytes per bucket

### VMGS Warnings
```json
{"VmId":"00000000-0000-0000-0000-000000000000","InfoString":"[onecore\\vm\\hcl\\fw\\vmgs\\VmgsDataStore.cpp:1365] HCL_STATUS = 0x00000008 - Failed to get VMGS file information of FileId(8)."}
```

### WIL Failures (Microsoft.Windows.HyperV.Storage)
Uses dedicated columns (`hresult`, `fileName`, `lineNumber`, `module`, `failureType`, `message`) instead of JSON Message:
- `hresult`: HRESULT error code (e.g., `2147942487` = `0x80070057` = E_INVALIDARG)
- `fileName`: Source file path (e.g., `"onecore\\vm\\dv\\storage\\scsi\\vdev\\dll\\synthstorattachment.cpp"`)
- `module`: DLL name (e.g., `"vmsynthstor.dll"`)

### NVMe Direct Telemetry (OperatingModeSwitch)
Uses dedicated columns (`vfLuid`, `serialNumber`, `tracePeriodStartTime`, `currentOperatingMode`, `slowEntryCount`, `fastEntryCount`, etc.) instead of JSON Message.

---

## 7. Sample Queries

### Query 1: Check Node for NVMe Direct Errors

First investigation step for NVMe Direct issues. Filters to Error and Critical levels only.

```kusto
let fn_faultTime = datetime(2023-07-21 13:28:43.7407789);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime;
let fn_nodeId = "5783dee0-a323-9369-00dd-bc0100ebef24";
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVStorageStackTable
| where ProviderName in ("Microsoft.Windows.HyperV.Storage.NvmeDirect",
    "Microsoft.Windows.HyperV.NvmeDirect.Telemetry",
    "Microsoft.Windows.HyperV.Storage.NvmeDirect2",
    "Microsoft.Windows.HyperV.Storage.NvmeDirect2.Activity")
| where NodeId == fn_nodeId
| where PreciseTimeStamp between(fn_startTime..fn_endTime)
| where Level < 3
| project PreciseTimeStamp, Pid, Tid, ProviderName, EventId, TaskName, Message, EventMessage, Level, Opcode
```

**CAUTION:** Some logged errors are benign. See the Common Failure Patterns section.

### Query 2: UseHardwareBarrier Is Closed (STATUS_LOCK_NOT_GRANTED)

Find instances where NVMe Direct rejects IOCTLs because hardware is already assigned to Guest.

```kusto
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVStorageStackTable
| where ProviderName == "Microsoft.Windows.HyperV.Storage.NvmeDirect"
| where Message matches regex "DevCtx [[:xdigit:]]+: IOCTL [[:xdigit:]]{8}: UseHardwareBarrier is closed"
| take 1
```

**IOCTL Values That Can Trigger This Error:**

| IOCTL | Value (Hex) | Value (Decimal) |
|---|---|---|
| IOCTL_NVME_DIRECT_READ_CONFIG_SPACE | 0x0022ec44 | 2288708 |
| IOCTL_NVME_DIRECT_WRITE_CONFIG_SPACE | 0x0022ec48 | 2288712 |
| IOCTL_NVME_DIRECT_READ_BAR | 0x0022ec4c | 2288716 |
| IOCTL_NVME_DIRECT_WRITE_BAR | 0x0022ec50 | 2288720 |
| IOCTL_NVME_DIRECT_ADD_ASQ_ENTRY | 0x0022ec58 | 2288728 |
| IOCTL_NVME_DIRECT_SWITCH_MODE | 0x0022ec5c | 2288732 |
| IOCTL_NVME_DIRECT_START_CONTROLLER | 0x0022ec68 | 2288744 |
| IOCTL_NVME_DIRECT_STOP_CONTROLLER | 0x0022ec6c | 2288748 |
| IOCTL_NVME_DIRECT_QUERY_MSIX_TABLE | 0x0022ec84 | 2288772 |
| IOCTL_NVME_DIRECT_ADMIN_PASS_THROUGH | 0x0022ed48 | 2288968 |

**Action:** Leave ICM comment: "A Host component is trying to trigger IOCTL_* to a NVMe device currently assigned to a Guest" with link to TSG. Transfer to scenario owner or RDOS/Azure Host OS SME - Virtualization (Hyper-V).

### Query 3: STATUS_INVALID_DEVICE_STATE Investigation (DevCtx-Scoped)

When you encounter `STATUS_INVALID_DEVICE_STATE { 0xc0000184, -1073741436 }`, grab the DevCtx pointer from the error message and search for the first error on that context. May need to widen the time window as the device might have been put in the wrong state long before the fault.

```kusto
let fn_faultTime = datetime(2023-08-23T20:21:32.4525103Z);
let fn_startTime = fn_faultTime - 5d;
let fn_endTime = fn_faultTime;
let fn_nodeId = "ae6a64b3-0131-cdac-8945-8e64cbc045ef";
let fn_devCtx = "FFFFD10CC42E44A0";
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVStorageStackTable
| where ProviderName in ("Microsoft.Windows.HyperV.Storage.NvmeDirect",
    "Microsoft.Windows.HyperV.NvmeDirect.Telemetry",
    "Microsoft.Windows.HyperV.Storage.NvmeDirect2",
    "Microsoft.Windows.HyperV.Storage.NvmeDirect2.Activity")
| where NodeId == fn_nodeId
| where PreciseTimeStamp between(fn_startTime..fn_endTime)
| where Message contains fn_devCtx
| where Level < 3
| project PreciseTimeStamp, Pid, Tid, ProviderName, EventId, TaskName, Message, EventMessage, Level, Opcode
| order by PreciseTimeStamp desc
```

**Note:** This error may be related to a **Micron firmware issue** (controllers not responding after Function Level Reset). Confirm by following the NVMe Direct Missing Disks TSG.

### Query 4: STATUS_DEVICE_UNRESPONSIVE

Find instances where the NVMe device is not responding (`{ 0xc000050a, -1073740534 }`).

```kusto
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVStorageStackTable
| where ProviderName == "Microsoft.Windows.HyperV.Storage.NvmeDirect"
| where Message has "c000050a"
| take 1
```

**Causes:** Bad PCIe switch, bad BIOS, bad firmware, failing hardware.  
**Next Steps:** Powercycle the machine → if fails, replace hardware → consult RDOS Incident Routing.

### Query 5: Find VMGS File Path from Storage Events

Determine the path to the VMGS file for a container by searching storage events.

```kusto
let fn_nodeId = '1cac291e-9a3b-91e5-2f92-487e02d95714';
let fn_containerId = '4a179a5f-68a9-40fd-8417-efb1e395b31d';
let fn_vmId = '77BE45D0-BE69-4072-9746-1DF41CDEFC3F';
let fn_faultTime = datetime(2023-11-13T02:20:00Z);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
cluster('azcore.centralus').database('Fa').HyperVStorageStackTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between (fn_startTime..fn_endTime)
| where Message contains fn_containerId
    or EventMessage contains fn_containerId
    or Message contains fn_vmId
    or EventMessage contains fn_vmId
| project PreciseTimeStamp, ProviderName, Pid, TaskName, Level, Opcode, Message, EventMessage
| order by PreciseTimeStamp desc
```

**VMGS File Path Patterns:**
- **Trusted Launch VMs:** `A:\[container id]_vmgs.vhd`
- **Non-Trusted Launch VMs:** `D:\<VM config directory>\[local vmid].vmgs`

### Query 6: VHD Error Investigation for a Node

Find VHD-related errors on a specific node.

```kusto
let fn_nodeId = "<node-id>";
let fn_faultTime = datetime(<fault-time>);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
cluster('azcore.centralus').database('Fa').HyperVStorageStackTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between(fn_startTime..fn_endTime)
| where ProviderName in ("Microsoft.Windows.HyperV.VhdmpTrace", "Microsoft-Windows-VHDMP", "Microsoft-Windows-VIRTDISK")
| where Level <= 2
| project PreciseTimeStamp, ProviderName, TaskName, EventId, Level, Message
| order by PreciseTimeStamp desc
```

### Query 7: Storage I/O Performance Analysis

Examine I/O latency histograms for a specific VHD file.

```kusto
let fn_nodeId = "<node-id>";
let fn_faultTime = datetime(<fault-time>);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime;
cluster('azcore.centralus').database('Fa').HyperVStorageStackTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between(fn_startTime..fn_endTime)
| where TaskName == "IoPerformance"
| extend parsed = parse_json(Message)
| extend vhdFile = tostring(parsed.vhdFileName), deviceName = tostring(parsed.deviceName)
| extend fileName = iff(isnotempty(vhdFile), vhdFile, deviceName)
| extend highLatencyCount = tolong(parsed.highLatencyIoCount)
| project PreciseTimeStamp, ProviderName, fileName, highLatencyCount, Message
| order by PreciseTimeStamp desc
```

### Query 8: VMGS Warning Events

Check for VMGS-related warnings that may indicate firmware or guest state issues.

```kusto
let fn_nodeId = "<node-id>";
let fn_faultTime = datetime(<fault-time>);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
cluster('azcore.centralus').database('Fa').HyperVStorageStackTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between(fn_startTime..fn_endTime)
| where TaskName == "VmgsWarning"
| extend parsed = parse_json(Message)
| project PreciseTimeStamp, tostring(parsed.VmId), tostring(parsed.InfoString)
| order by PreciseTimeStamp desc
```

### Query 9: WIL Failures in Storage Stack

Investigate Windows Implementation Library failures in storage components.

```kusto
let fn_nodeId = "<node-id>";
let fn_faultTime = datetime(<fault-time>);
let fn_startTime = fn_faultTime - 1h;
let fn_endTime = fn_faultTime + 1h;
cluster('azcore.centralus').database('Fa').HyperVStorageStackTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between(fn_startTime..fn_endTime)
| where ProviderName == "Microsoft.Windows.HyperV.Storage"
| where isnotempty(hresult)
| project PreciseTimeStamp, hresult, fileName, lineNumber, module, failureType, message
| order by PreciseTimeStamp desc
```

---

## 7. Cross-Table Query Patterns

### Correlate Storage Events with Hypervisor Events

Join storage failures with hypervisor events to understand if a host-level event preceded storage issues.

```kusto
let fn_nodeId = "<node-id>";
let fn_startTime = datetime(<start>);
let fn_endTime = datetime(<end>);
let storageErrors = cluster('azcore.centralus').database('Fa').HyperVStorageStackTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between(fn_startTime..fn_endTime)
| where Level <= 2
| project PreciseTimeStamp, ProviderName, TaskName, Message, Level;
let hypervisorEvents = cluster('azcore.centralus').database('Fa').HyperVHypervisorTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between(fn_startTime..fn_endTime)
| where Level <= 2
| project PreciseTimeStamp, ProviderName, TaskName, Message, Level;
union storageErrors, hypervisorEvents
| order by PreciseTimeStamp asc
```

### Cross-Reference with HyperVWorkerTable

Correlate VM worker process events with storage events for the same VM.

```kusto
let fn_nodeId = "<node-id>";
let fn_vmId = "<vm-id>";
let fn_startTime = datetime(<start>);
let fn_endTime = datetime(<end>);
cluster('azcore.centralus').database('Fa').HyperVStorageStackTable
| where NodeId == fn_nodeId
| where PreciseTimeStamp between(fn_startTime..fn_endTime)
| where Message contains fn_vmId or EventMessage contains fn_vmId
| project PreciseTimeStamp, Source="Storage", ProviderName, TaskName, Level, Message
| union (
    cluster('azcore.centralus').database('Fa').HyperVWorkerTable
    | where NodeId == fn_nodeId
    | where PreciseTimeStamp between(fn_startTime..fn_endTime)
    | where Message contains fn_vmId or EventMessage contains fn_vmId
    | project PreciseTimeStamp, Source="Worker", ProviderName, TaskName, Level, Message
)
| order by PreciseTimeStamp asc
```

---

## 8. Investigation Playbook

### NVMe Direct Issues

1. **Run Query 1** (Check for NVMe Direct errors) around the fault time — filter `Level < 3`
2. Review `Message` field and error codes using the flowchart in the [NVMe Direct Errors TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/deviceassignment/nvme-direct-errors)
3. Check for **benign errors** (see Common Failure Patterns) — do not escalate on those
4. For `STATUS_INVALID_DEVICE_STATE` → use **Query 3** with the DevCtx pointer scoped over a wider time window (up to 5 days back)
5. For `STATUS_DEVICE_UNRESPONSIVE` → powercycle, then hardware replacement if needed
6. For `UseHardwareBarrier is closed` → ICM comment + transfer to scenario owner

### VMGS (Virtual Machine Guest State) Issues

1. Use **Query 5** to find the VMGS file path for the container
2. Check **Query 8** for VmgsWarning events
3. **VMGS file path patterns:**
   - Trusted Launch VMs: `A:\[container id]_vmgs.vhd`
   - Non-Trusted Launch VMs: `D:\<VM config dir>\[local vmid].vmgs`
4. Alternative: Run `vmadmin querysettings [container id]` on the node to get `GuestStateFileRoot` and `GuestStateFileName`

**VMGS Well-Known File IDs (VMGSv3):**

| File ID | Name | Value |
|---|---|---|
| FILE_TABLE | File Table | 0 |
| BIOS_NVRAM | BIOS NVRAM | 1 |
| TPM_PPI | TPM PPI | 2 |
| TPM_NVRAM | TPM NVRAM | 3 |
| RTC_SKEW | RTC Skew | 4 |
| ATTEST | Attest | 5 |
| KEY_PROTECTOR | Key Protector | 6 |
| VM_UNIQUE_ID | VM Unique ID | 7 |
| GUEST_FIRMWARE | Guest Firmware | 8 |
| CUSTOM_UEFI | Custom UEFI | 9 |
| GUEST_WATCHDOG | Guest Watchdog | 10 |
| HW_KEY_PROTECTOR | HW Key Protector | 11 |
| GUEST_SECRET_KEY | Guest Secret Key | 13 |
| HIBERNATION_FIRMWARE | Hibernation Firmware | 14 |
| EXTENDED_FILE_TABLE | Extended File Table | 63 |

**VM Types and VMGS Behavior:**
- **Trusted Launch V1 / Confidential VMs:** VMGS provisioned by Host Agent or CPS using VmgsTool; uses HCL (OpenHCL or legacy); VMGSv3 format; located on shared storage associated with OS disk
- **Non-Trusted Launch OpenHCL VMs:** VMGS created by VMMS, provisioned by OpenHCL on first boot; VMGSv3 format; located on node-local storage (lost on deallocation)
- **Gen1/Gen2 VMs (version > 8.0, no HCL):** VMGS created/provisioned by VMMS and VMWP; VMGSv1 format; located on node-local storage

**VMGS Failure Types:**
- **VmgsTool Failures:** During TVM/CVM deployment, Host Agent or CPS uses VmgsTool to create/encrypt the VMGS. Operations can fail or timeout, leaving VMGS in bad state
- **HCL VMGS Failures:** During HCL initialization, VMGS headers are checked for corruption. HCL may crash if it cannot decrypt using available methods

### VHD/VHDX Errors

1. Use **Query 6** to find VHD errors on the node
2. Parse `Message` JSON for `Source`, `Line`, and `Message` fields
3. Look for NTSTATUS codes in the message (e.g., `0xC0000034` = STATUS_OBJECT_NAME_NOT_FOUND)
4. Check if errors are related to file open failures, resiliency timeout issues, or backing file problems

### Storage I/O Latency

1. Use **Query 7** to examine I/O performance histograms
2. Check `highLatencyIoCount` for elevated values
3. Analyze latency bucket distribution — IOs in buckets ≥ `1000000000` (1 second) indicate severe latency

### WIL Failures

1. Use **Query 9** to find WIL failures
2. Key fields: `hresult`, `fileName`, `module`, `message`
3. Common: `0x80070057` (E_INVALIDARG) in `vmsynthstor.dll` from `synthstorattachment.cpp`

---

## 9. Common Failure Patterns

### Benign Errors (Do Not Indicate a Problem)

| Error Pattern | Description | Action |
|---|---|---|
| `NvmdRequestUnmarkCancelable - STATUS_CANCELLED { 0xc0000120, -1073741536 }` | Both driver and worker process cancel the same request. If worker cancels first, driver logs this warning. Only happens during VM PowerOff. | Continue investigating other errors. |
| `NVMe Async Event-Namespace Change` / `Async Event Request` | Sometimes logged as "warning" due to Guest OS activity. | Continue investigating other errors. |
| `Unknown IOCTL` (e.g., `"DevCtx ...: [---] Unknown Ioctl 41018"`) | NVMe Direct received a command it does not recognize. Not indicative of an error in the NVMe Direct stack. Can be a symptom of incorrect behavior elsewhere but is benign to correct Host OS operation. Use `!ioctldecode` in WinDbg to determine the IOCTL. | Continue investigating other errors. |

### Errors Requiring Investigation

| Error Pattern | Status Code | Description | Next Steps |
|---|---|---|---|
| `IOCTL_NVME_DIRECT_* — UseHardwareBarrier is closed` | STATUS_LOCK_NOT_GRANTED | Host component calling into NVMe Direct at wrong time (hardware already allocated to Guest). | ICM comment + transfer to scenario owner or RDOS/Azure Host OS SME - Virtualization (Hyper-V). |
| `STATUS_INVALID_DEVICE_STATE` | `0xc0000184` / `-1073741436` | Driver not in correct state to process request. May be related to Micron FLR firmware issue (missing NVMe Direct disks). | Check NVMe Direct Missing Disks TSG. Use DevCtx-scoped query (Query 3) to find root cause. |
| `STATUS_DEVICE_UNRESPONSIVE` | `0xc000050a` / `-1073740534` | Device not responding. Bad PCIe switch, BIOS, firmware, or failing hardware. | Powercycle machine. If fails, replace hardware. Consult RDOS Incident Routing. |

---

## 10. When to Use This Table

**Use HyperVStorageStackTable when investigating:**
- NVMe Direct device assignment errors or missing disks in VMs
- VHD/VHDX file operation failures (open, close, attach, detach)
- VMGS (Virtual Machine Guest State) file issues (corruption, provisioning failures, missing file IDs)
- Storage I/O latency or performance regressions (IoPerformance histograms)
- StorVSP / synthetic storage controller issues
- WIL failures in storage components (`vmsynthstor.dll`, etc.)
- Emulated (IDE) storage controller issues (rare)
- Virtual PMEM issues (very rare)

**Do NOT use this table for:**
- Hypervisor-level issues (use `HyperVHypervisorTable`)
- VM worker process issues (use `HyperVWorkerTable`)
- VMMS management plane issues (use `HyperVVmmsTable`)
- vPCI / device assignment non-storage issues (use `HyperVVPciTable`)
- Guest OS-internal storage issues (use guest-side telemetry)

---

## 11. Escalation Paths

| Scenario | IcM Queue |
|---|---|
| General NVMe Direct issues | RDOS / zHYP SME DAS (HYP SME use only) |
| Specific NVMe Direct issues | Contact: `nvmedirect` |
| Device assignment issues | Contact: `vpcidev` |
| ASAP (storage acceleration) | Host Storage Acceleration / Triage |
| MANA (networking) | Host Networking / Triage |
| Suspicious NVMe errors not covered by TSG | zHYP SME VCP Devices and Storage (HYP SME use only) — include query links and results |

---

## Source Pages

1. [Hyper-V Kusto Queries](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/hyperv-kusto-queries)
2. [VMGS TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/tsg-vmgs)
3. [NVMe Direct Errors TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/deviceassignment/nvme-direct-errors)
