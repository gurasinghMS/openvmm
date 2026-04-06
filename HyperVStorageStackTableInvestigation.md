# HyperVStorageStackTable Investigation Guide

> Distilled from RDOS Livesite EngHub documentation.
> Covers: HyperVStorageStackTable in azcore.centralus / Fa

---

## Table Overview

**HyperVStorageStackTable** contains a filtered view of events and traces of various Hyper-V's Storage Virtualization providers. It collects them in a single table with the payload as a JSON blob in column **Message**. This way, the schema is kept short given the broad number of providers contained in this table. The complete set of providers collected is found in the MdsFa.xml file.

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
| `ProviderName` | ETW provider that emitted the event (see Provider Names below) |
| `EventId` | Numeric event identifier |
| `TaskName` | Task name associated with the event |
| `Message` | **JSON blob** containing event payload |
| `EventMessage` | Formatted event message string |
| `Level` | Event severity level (1=Critical, 2=Error, 3=Warning, 4=Informational, 5=Verbose) |
| `Opcode` | Operation code for the event |
| `Pid` | Process ID |
| `Tid` | Thread ID |

---

## Provider Names

The following ETW providers are collected into HyperVStorageStackTable (from NVMe Direct investigations):

| Provider Name | Description |
|---|---|
| `Microsoft.Windows.HyperV.Storage.NvmeDirect` | Primary NVMe Direct storage provider |
| `Microsoft.Windows.HyperV.NvmeDirect.Telemetry` | NVMe Direct telemetry events |
| `Microsoft.Windows.HyperV.Storage.NvmeDirect2` | NVMe Direct v2 storage provider |
| `Microsoft.Windows.HyperV.Storage.NvmeDirect2.Activity` | NVMe Direct v2 activity tracing |

Additional providers for StorVSP, SCSI, VHD, and other storage virtualization components are defined in MdsFa.xml.

---

## All Kusto Queries

### Query 1: Check Node for NVMe Direct Errors

**Source:** [NVMe Direct Errors TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/deviceassignment/nvme-direct-errors)

**Purpose:** Check if NVMe Direct errors are being reported around the fault time. This is the first investigation step for NVMe Direct issues.

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

**Notes:**
- Filters to `Level < 3` (Error and Critical only).
- **CAUTION:** Some logged errors are benign. See the Common Failure Patterns section below.

---

### Query 2: IOCTL_NVME_DIRECT — UseHardwareBarrier Is Closed (STATUS_LOCK_NOT_GRANTED)

**Source:** [NVMe Direct Errors TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/deviceassignment/nvme-direct-errors)

**Purpose:** Find instances where the error message `"DevCtx %p: IOCTL %08x: UseHardwareBarrier is closed"` appears, indicating STATUS_LOCK_NOT_GRANTED.

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

**Meaning:** The request is coming in at a time when the driver is no longer allowing the Host to issue commands to the hardware. This can happen if the hardware is already allocated to the Guest.

**Action:** Leave ICM comment: "A Host component is trying to trigger IOCTL_* to a NVMe device currently assigned to a Guest" with link to the TSG. Transfer to scenario owner. If scenario not clear, transfer to RDOS/Azure Host OS SME - Virtualization (Hyper-V).

---

### Query 3: STATUS_INVALID_DEVICE_STATE Investigation (DevCtx-Scoped)

**Source:** [NVMe Direct Errors TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/deviceassignment/nvme-direct-errors)

**Purpose:** When you encounter `STATUS_INVALID_DEVICE_STATE { 0xc0000184, -1073741436 }`, grab the DevCtx pointer from the error message and search for the first error on that context. You may need to widen the time window (`fn_startTime`) as the device might have been put in the wrong state long before the fault.

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

**Notes:**
- The `fn_devCtx` is the pointer immediately after "DevCtx" in the error message (e.g., `DevCtx FFFFD10CC42E44A0: NvmdIoctlIssueNvmeCommand fails with status c0000184`).
- This error may be seen in conjunction with a known firmware issue related to **Micron controllers not responding after Function Level Reset (FLR)**. This issue is mostly characterized by missing NVMe Direct disks in the VM. Confirm if related by following the NVMe Direct Missing Disks TSG.
- If another error is not indicated, engage the SME Team (see Escalation Paths below).

---

### Query 4: STATUS_DEVICE_UNRESPONSIVE

**Source:** [NVMe Direct Errors TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/deviceassignment/nvme-direct-errors)

**Purpose:** Find instances where the device is not responding (STATUS_DEVICE_UNRESPONSIVE `{ 0xc000050a, -1073740534 }`).

```kusto
cluster('azcore.centralus.kusto.windows.net').database('Fa').HyperVStorageStackTable
| where ProviderName == "Microsoft.Windows.HyperV.Storage.NvmeDirect"
| where Message has "c000050a"
| take 1
```

**Causes:** Bad PCIe switch, bad BIOS, bad firmware, failing hardware, etc.

**Next Steps:**
1. Powercycle the machine and see if the device becomes responsive.
2. If powercycle fails, the hardware will need to be replaced.
3. Consult the RDOS Incident Routing to help facilitate this.

---

### Query 5: Find VMGS File Path from Storage Events

**Source:** [VMGS TSG](https://eng.ms/docs/cloud-ai-platform/azure-core/azure-compute/kvs/rdos/livesite/tsg/virtualization/firmware/tsg-vmgs)

**Purpose:** The HyperVStorageStackTable often logs events related to the VMGS file and includes its path in the event. Use this query to determine the path to the VMGS file for a container.

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

**Alternative:** On the node, run `vmadmin querysettings [container id]` to list `GuestStateFileRoot` and `GuestStateFileName`.

---

## Storage-Specific Investigation Patterns

### VMGS (Virtual Machine Guest State)

The VMGS file contains TPM state, UEFI BIOS NVRAM variables, and other information for consistent VM behavior across reboots. Well-known file IDs in VMGSv3:

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
- **Trusted Launch V1 / Confidential VMs:** VMGS provisioned by Host Agent or CPS using VmgsTool; uses HCL (OpenHCL or legacy); VMGSv3 format; located on shared storage associated with OS disk.
- **Non-Trusted Launch OpenHCL VMs:** VMGS created by VMMS, provisioned by OpenHCL on first boot; VMGSv3 format; located on node-local storage (lost on deallocation).
- **Gen1/Gen2 VMs (version > 8.0, no HCL):** VMGS created/provisioned by VMMS and VMWP; VMGSv1 format; located on node-local storage.

**VMGS Failure Types:**
- **VmgsTool Failures:** During TVM/CVM deployment, Host Agent or CPS uses VmgsTool to create/encrypt the VMGS. Operations can fail or timeout, leaving VMGS in bad state.
- **HCL VMGS Failures:** During HCL initialization, VMGS headers are checked for corruption. HCL may crash if it cannot decrypt using available methods.

### NVMe Direct

Use the NVMe Direct queries above to check for errors. Key investigation flow:
1. Run Query 1 (Check for NVMe Direct errors) around fault time.
2. Review Message field and error codes using the flowchart in the NVMe Direct Errors TSG.
3. Determine if error is benign or requires further investigation.

### StorVSP / SCSI

Events from StorVSP and SCSI providers are also collected in this table via MdsFa.xml. Filter by relevant `ProviderName` values for StorVSP-specific investigations.

### VHD

VHD-related events (open, close, read/write errors) are captured in this table. Look for events related to virtual disk operations in the `Message` JSON.

---

## Common Failure Patterns

### Benign Errors (Do Not Indicate a Problem)

| Error Pattern | Description | Action |
|---|---|---|
| `NvmdRequestUnmarkCancelable - STATUS_CANCELLED { 0xc0000120, -1073741536 }` | Both driver and worker process cancel the same request. If worker cancels first, driver logs this warning. Only happens during VM PowerOff. | Continue investigating other errors. |
| `NVMe Async Event-Namespace Change` / `Async Event Request` | Sometimes logged as "warning" due to Guest OS activity. | Continue investigating other errors. |
| `Unknown IOCTL` (e.g., `"DevCtx FFFFD609015CC3D0: [---] Unknown Ioctl 41018"`) | NVMe Direct received a command it does not recognize. Not indicative of an error in the NVMe Direct stack. Can be a symptom of incorrect behavior elsewhere but is benign to correct Host OS operation. Use `!ioctldecode` in WinDbg to determine the IOCTL. | Continue investigating other errors. |

### Errors Requiring Investigation

| Error Pattern | Status Code | Description | Next Steps |
|---|---|---|---|
| `IOCTL_NVME_DIRECT_* — UseHardwareBarrier is closed` | STATUS_LOCK_NOT_GRANTED | Host component calling into NVMe Direct at wrong time (hardware already allocated to Guest). | ICM comment + transfer to scenario owner or RDOS/Azure Host OS SME - Virtualization (Hyper-V). |
| `STATUS_INVALID_DEVICE_STATE` | `0xc0000184` / `-1073741436` | Driver not in correct state to process request. May be related to Micron FLR firmware issue (missing NVMe Direct disks). | Check NVMe Direct Missing Disks TSG. Use DevCtx-scoped query (Query 3) to find root cause. |
| `STATUS_DEVICE_UNRESPONSIVE` | `0xc000050a` / `-1073740534` | Device not responding. Bad PCIe switch, BIOS, firmware, or failing hardware. | Powercycle machine. If fails, replace hardware. Consult RDOS Incident Routing. |

---

## Escalation Paths

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

**Maintainer:** Contact `hypsme` | IcM queue: RDOS/Azure Host OS SME - Virtualization (Hyper-V)
