---
name: kusto-explorer
description: "Query and explore Kusto clusters, databases, and tables related to OpenVMM/OpenHCL/Underhill testing and servicing. Covers wdgeventstore cluster (CCA database, UnderhillTestServicingQualityMV), azcore.centralus cluster (Fa database, UnderhillEventTable with detailed log data), and step-by-step query planning and verification."
---

# Kusto Explorer Skill

This skill is invoked **only** when the user asks specific questions about Kusto data related to OpenVMM, OpenHCL, or Underhill testing and servicing metrics.

## READ-ONLY OPERATIONS ONLY

**CRITICAL: This skill performs READ-ONLY operations exclusively.**

**FORBIDDEN operations:**
- Never create, modify, or delete tables or materialized views
- Never insert, update, or delete data in any table
- Never execute any KQL command that modifies data (`.create`, `.alter`, `.drop`, `.set`, `.append`, `.ingest`, etc.)
- Never create or edit table documentation files
- Never attempt to add new tables/views to the skill documentation

**Allowed operations:**
- Query data using `SELECT`-style KQL queries
- List tables and views using `.show` commands (read-only admin commands)
- Retrieve schemas using `| getschema`
- Execute read-only queries with filters, aggregations, and transformations

## Prerequisites & Required Permissions

**ALWAYS start by acquiring permissions before executing any workflow:**

1. **MCP Kusto Tools** — Verify you have access to:
   - `mcp_kusto_list_tables`
   - `mcp_kusto_get_table_schema`
   - `mcp_kusto_execute_query`

2. **Authentication** — The MCP server must be authenticated to the Kusto cluster. If queries fail with authentication errors, inform the user that they need to configure the Kusto MCP server with proper credentials.

3. **Network Access** — Ensure connectivity to `wdgeventstore.kusto.windows.net`.

**If any required permission is missing, ask the user to grant it before proceeding.**

## Available Tables and Views

This skill has detailed documentation for the following Kusto resources:

### 1. **UnderhillTestServicingQualityMV** (Materialized View)
- **Location:** `wdgeventstore.kusto.windows.net` → `CCA` database
- **Purpose:** Servicing operation metrics, performance tracking (boot times, blackout durations), firmware version transitions
- **Documentation:** See [wdgeventstore_CCA_UnderhillTestServicingQualityMV.md](wdgeventstore_CCA_UnderhillTestServicingQualityMV.md)

### 2. **UnderhillEventTable** (Table)
- **Location:** `azcore.centralus.kusto.windows.net` → `Fa` database
- **Purpose:** Detailed runtime event logs from OpenHCL/Underhill components (ETW traces, Rust tracing logs, UEFI firmware logs)
- **Volume:** ~5 billion events/day; retention ~2+ months
- **Documentation:** See [azcore.centralus_Fa_UnderhillEventTable.md](azcore.centralus_Fa_UnderhillEventTable.md)

### 3. **HyperVHypervisorTable** (Table)
- **Location:** `azcore.centralus.kusto.windows.net` → `Fa` database
- **Purpose:** Hypervisor-level ETW events (partition lifecycle, VP scheduling, memory management, HYPERVISOR_ERROR bugchecks)
- **Documentation:** See [azcore.centralus_Fa_HyperVHypervisorTable.md](azcore.centralus_Fa_HyperVHypervisorTable.md)

### 4. **HyperVStorageStackTable** (Table)
- **Location:** `azcore.centralus.kusto.windows.net` → `Fa` database
- **Purpose:** Storage stack telemetry from NVMe Direct, VHD/VHDX, VMGS, and IO performance providers (~34.8B events/day)
- **Documentation:** See [azcore.centralus_Fa_HyperVStorageStackTable.md](azcore.centralus_Fa_HyperVStorageStackTable.md)

### 5. **HyperVVmmsTable** (Table)
- **Location:** `azcore.centralus.kusto.windows.net` → `Fa` database
- **Purpose:** VMMS management service telemetry — VM lifecycle, live migration, servicing dispatch, stop-container (~148B events/day)
- **Documentation:** See [azcore.centralus_Fa_HyperVVmmsTable.md](azcore.centralus_Fa_HyperVVmmsTable.md)

### 6. **HyperVVPciTable** (Table)
- **Location:** `azcore.centralus.kusto.windows.net` → `Fa` database
- **Purpose:** Virtual PCI bus telemetry — device assignment/teardown, MANA/NVMe/GPU pass-through, VPCI proxy operations (~38B events/day)
- **Documentation:** See [azcore.centralus_Fa_HyperVVPciTable.md](azcore.centralus_Fa_HyperVVPciTable.md)

### 7. **HyperVWorkerTable** (Table)
- **Location:** `azcore.centralus.kusto.windows.net` → `Fa` database
- **Purpose:** VM worker process (vmwp.exe) telemetry — VM start/stop, crashes, UEFI boot events, Underhill servicing, live migration (~132B events/day)
- **Documentation:** See [azcore.centralus_Fa_HyperVWorkerTable.md](azcore.centralus_Fa_HyperVWorkerTable.md)

**Typical Workflow:**
1. Use **UnderhillTestServicingQualityMV** to identify failed servicing operations or performance regressions
2. Use **UnderhillEventTable** to drill into detailed Underhill/OpenHCL logs for root cause analysis
3. Use **HyperVWorkerTable** for VM lifecycle events (start, stop, crash, servicing, boot)
4. Use **HyperVVmmsTable** for management-level operations (live migration, stop-container, servicing dispatch)
5. Use **HyperVVPciTable** to investigate device assignment issues (MANA, NVMe, GPU)
6. Use **HyperVStorageStackTable** for storage-specific issues (VHD errors, NVMe Direct, IO latency, VMGS)
7. Use **HyperVHypervisorTable** for hypervisor-level issues (partition creation/deletion, VP config, bugchecks)

## Query Philosophy: Step-by-Step Verification

**NEVER rush into complex queries.** Follow this iterative approach:

### 1. Start Small
- Query a **single source** at a time
- Retrieve **minimal rows** (use `| take 5` or `| take 10`)
- Use `| getschema` to understand column structure first

### 2. Analyze Before Proceeding
- Look at the actual data returned
- Verify column names, types, and sample values
- Check for nulls, unexpected formats, or anomalies

### 3. Plan the Next Step
- Write down what transformation you want to apply
- **Document what you EXPECT to see** after the transformation
- Apply the transformation to the **same small dataset**

### 4. Verify Each Step
- Compare actual output to expected output
- **If they match:** proceed to the next step
- **If they don't match:** PAUSE, diagnose the issue, fix it, and re-verify before moving forward

### 5. Document As You Go
- Keep notes on what each query does
- Track the columns being used
- Record any insights or patterns discovered

### Example: Multi-Step Transformation Workflow

**Bad approach (don't do this):**
```kql
// Trying to do everything at once - HIGH RISK OF ERRORS
MyTable
| extend DataList = parse_json(RawData)
| mv-expand DataItem = DataList
| extend ProcessedValue = tostring(DataItem.field1)
| summarize Results = make_list(ProcessedValue) by Category
| project Category, PackedResults = pack_all()
```

**Good approach (do this instead):**

**Step 1: Get sample data**
```kql
MyTable | take 5 | project RawData
```
**Expected:** See what RawData looks like (JSON string? Structured?)

**Step 2: Parse the data**
```kql
MyTable | take 5 | extend DataList = parse_json(RawData) | project DataList
```
**Expected:** DataList should be an array/object structure

**Step 3: Expand the data**
```kql
MyTable | take 5
| extend DataList = parse_json(RawData)
| mv-expand DataItem = DataList
| project DataItem
```
**Expected:** Each array element becomes a separate row

**Step 4: Extract specific fields**
```kql
MyTable | take 5
| extend DataList = parse_json(RawData)
| mv-expand DataItem = DataList
| extend ProcessedValue = tostring(DataItem.field1)
| project ProcessedValue
```
**Expected:** Clean string values from field1

**Step 5: Only after ALL steps verify correctly, build the full query**

## When to Give Up

Stop and inform the user if:

1. **Authentication fails repeatedly** — the MCP server needs reconfiguration
2. **Table/database doesn't exist after verification** — user may have provided incorrect name
3. **Query syntax errors persist after 3 attempts** — ask user for clarification or simplification
4. **Query times out repeatedly** — the query may be too expensive; suggest filtering or sampling
5. **Schema doesn't contain expected columns** — document findings and ask user to verify requirements

## Known Kusto Clusters & Databases

### wdgeventstore Cluster

**Endpoint:** `https://wdgeventstore.kusto.windows.net`

#### Database: CCA

**Purpose:** Cirrus Compute Analytics — telemetry, test results, and servicing data for Azure compute workloads including Underhill/OpenVMM/OpenHCL.

**Connection:**
```
Cluster: https://wdgeventstore.kusto.windows.net
Database: CCA
```

## Table-Specific Documentation

**Table and materialized view details are stored in dedicated files within this skill folder.**

### File Naming Convention

Each table/view has its own documentation file named:
```
<ClusterName>_<DatabaseName>_<TableOrViewName>.md
```

**Examples:**
- `wdgeventstore_CCA_UnderhillTestServicingQualityMV.md`
- `wdgeventstore_CCA_MyTableName.md`

### When to Load Table-Specific Files

**BEFORE querying a specific table or materialized view**, check if a dedicated documentation file exists:

1. **Construct the file path:**
   ```
   /home/gurasingh/openvmm_forked/.github/skills/kusto-explorer/<cluster>_<database>_<table>.md
   ```

2. **Read the file** (if it exists) to get:
   - Complete schema with column descriptions
   - Sample queries specific to that table/view
   - Common use cases and query patterns
   - Important columns and filtering tips

3. **If the file doesn't exist:**
   - Use the general workflow below
   - Start with `.show tables` or `.show materialized-views`
   - Use `| getschema` to discover the schema
   - Build queries incrementally

### Currently Documented Tables/Views

- **UnderhillTestServicingQualityMV** — Underhill servicing quality metrics and performance data
  - File: `wdgeventstore_CCA_UnderhillTestServicingQualityMV.md`
- **UnderhillEventTable** — Detailed runtime event logs from OpenHCL/Underhill components (ETW traces, Rust tracing logs, UEFI firmware logs)
  - File: `azcore.centralus_Fa_UnderhillEventTable.md`
- **HyperVHypervisorTable** — ETW events from the Hyper-V hypervisor (hvix64/hvax64/hvaa64), covering partition lifecycle, VP scheduling, memory management, and HYPERVISOR_ERROR bugchecks
  - File: `azcore.centralus_Fa_HyperVHypervisorTable.md`
- **HyperVStorageStackTable** — Storage stack telemetry from 15 ETW providers including NVMe Direct, VHD/VHDX, VMGS, and IO performance histograms
  - File: `azcore.centralus_Fa_HyperVStorageStackTable.md`
- **HyperVVmmsTable** — VMMS (vmms.exe) management service telemetry covering VM lifecycle, live migration, servicing dispatch, and stop-container operations
  - File: `azcore.centralus_Fa_HyperVVmmsTable.md`
- **HyperVVPciTable** — Virtual PCI (vPCI) bus telemetry covering device assignment, teardown, MANA/NVMe/GPU pass-through, and VPCI proxy operations
  - File: `azcore.centralus_Fa_HyperVVPciTable.md`
- **HyperVWorkerTable** — VM worker process (vmwp.exe) telemetry covering VM start/stop, crashes, UEFI boot events, Underhill servicing, live migration, and device emulation
  - File: `azcore.centralus_Fa_HyperVWorkerTable.md`

## Workflow: Querying Kusto Data

### Phase 1: Acquire Permissions
1. Confirm MCP Kusto tools are available
2. Verify authentication is configured
3. If missing permissions, request them from the user

### Phase 2: Plan Your Query
1. Identify the target cluster, database, and table/view
2. Determine what columns you need
3. Decide on filters (time ranges, status codes, etc.)
4. Estimate result size and plan to limit rows if necessary

### Phase 3: Execute Step-by-Step
1. **Verify the table/view exists:**
   ```kql
   // For tables:
   .show tables | where TableName == "TargetTable"
   
   // For materialized views:
   .show materialized-views | where Name == "TargetView"
   ```

2. **Get the schema:**
   ```kql
   // For tables, use MCP tool mcp_kusto_get_table_schema or:
   TargetTable | getschema
   
   // For materialized views, MUST use query (MCP tool doesn't work):
   TargetView | getschema
   ```

3. **Retrieve a small sample:**
   ```kql
   TargetTable | take 5
   ```

4. **Analyze the sample** — check data quality, column values, nulls

5. **Build the query incrementally:**
   - Add filters one at a time
   - Verify after each addition
   - Test transformations on small datasets first
   - Document expected vs actual results

6. **Scale up only after verification:**
   - Remove `| take` limits
   - Expand time ranges
   - Run full aggregations

### Phase 4: Document Findings
- Summarize what was discovered
- Note any anomalies or interesting patterns
- Record the final query for future reference

## Tool Usage

**Always use the MCP Kusto tools:**
- `mcp_kusto_list_tables` — list all tables in a database (note: does NOT include materialized views)
- `mcp_kusto_get_table_schema` — get column names and types for a table (note: does NOT work for materialized views)
- `mcp_kusto_execute_query` — execute KQL queries (works for both tables and materialized views)

**For materialized views:**
- Use `.show materialized-views` query to list them
- Use `ViewName | getschema` query to get schema (the MCP schema tool won't work)
- Use `mcp_kusto_execute_query` for all data queries

**Never attempt to:**
- Use REST APIs directly
- Construct connection strings manually
- Run queries through terminal commands (unless debugging MCP issues)

## Error Handling

**Common errors and solutions:**

| Error | Likely Cause | Solution |
|-------|--------------|----------|
| `socket hang up` | Network timeout or cluster unavailable | Retry once; if persists, inform user |
| `Request failed with status code 400` | Invalid KQL syntax or table name | Check table name exists; verify KQL syntax |
| `Request failed with status code 401` | Authentication failure | User needs to reconfigure MCP server credentials |
| `Request failed with status code 403` | Insufficient permissions | User needs database read permissions |
| `Table not found` | Wrong database or table name | List tables with `.show tables` and verify |

## Summary

**This skill is for Kusto data exploration related to OpenVMM/OpenHCL/Underhill testing and servicing.**

**Core principles:**
- Always acquire permissions first
- Plan queries before executing
- Execute step-by-step with verification at each stage
- Keep queries simple and focused
- Document as you go
- Use MCP Kusto tools exclusively

**Workflow:**
1. Before querying a known table/view, check for a dedicated `<cluster>_<database>_<table>.md` file in this folder
2. Load the file to get schema, sample queries, and best practices
3. If no file exists, use the step-by-step discovery workflow to explore the data
4. Answer the user's question using READ-ONLY queries only

**Currently documented sources:**
- **Cluster:** wdgeventstore (`https://wdgeventstore.kusto.windows.net`)
- **Database:** CCA
- **Views:** UnderhillTestServicingQualityMV

**When invoked:** Only when user asks specific questions about Kusto data for these systems.
