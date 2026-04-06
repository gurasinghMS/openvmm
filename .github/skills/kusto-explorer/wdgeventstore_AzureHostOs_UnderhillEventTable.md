# UnderhillEventTable (wdgeventstore — DEPRECATED COPY)

**Type:** Table  
**Cluster:** `https://wdgeventstore.kusto.windows.net`  
**Database:** `AzureHostOs`  
**Full Path:** `wdgeventstore.kusto.windows.net` → `AzureHostOs` → `UnderhillEventTable`

---

> ## ⛔ DO NOT USE THIS TABLE
>
> **Always use `azcore.centralus.kusto.windows.net` → `Fa` → `UnderhillEventTable` instead.**
>
> This is a deprecated copy with an inferior schema. It is missing critical columns that the canonical table has:
> - `Region`
> - `DataCenter`
> - `AutopilotEnvironment`
> - `NodeIdentity`
> - `DeviceId`
>
> Some legacy TSG documentation and older queries reference this copy. If you encounter such a query, **rewrite it to use `azcore.centralus / Fa`** instead. The core event data (Message, VmId, VmName, NodeId, Level, PreciseTimeStamp) is identical between both copies.
>
> For the full documentation, schema, sample queries, investigation playbooks, and best practices, see **`azcore.centralus_Fa_UnderhillEventTable.md`**.

---

## Schema

This copy has the following columns that **differ** from the canonical `azcore.centralus / Fa` version:

| Column Name | Type | Description |
|-------------|------|-------------|
| `Role` | string | Role identifier (not present in canonical copy) |
| `Hostname` | string | Node hostname (not present in canonical copy) |
| `__IsTrusted__` | string | Trust classification of the event source (not present in canonical copy) |

These three columns provide no meaningful advantage over the richer metadata available in the canonical table.

## Why This Copy Exists

Historical reasons. Some Underhill TSG documentation was written when `wdgeventstore` was the primary ingestion path. The data has since been made available on `azcore.centralus / Fa` with a better schema. The `wdgeventstore` copy is retained for backward compatibility but should not be used for new queries.

## Other Useful Tables on wdgeventstore

The `wdgeventstore` cluster hosts other tables and functions that ARE useful and do not have equivalents elsewhere:

| Identifier | Database | Table/Function | Purpose |
|---|---|---|---|
| `wdgeventstore_CCA_UnderhillTestServicingQualityMV` | CCA | Servicing quality materialized view | Servicing outcomes, blackout times, firmware versions |
| `wdgeventstore_CCA_UnderhillServicingExecutionData` | CCA | Servicing execution details | Stuck servicing investigation |
| `wdgeventstore_CCA_GetUnderhillBinaryCommitHash` | CCA | Function: DLL version → git commit | Map Underhill version to source code |
| `wdgeventstore_HostOSDeploy_AnyHostUpdateOnNode` | HostOSDeploy | Function: check host OS updates | Correlate faults with host updates |

These are separate resources — they do NOT query UnderhillEventTable. Use them freely alongside the canonical `azcore.centralus / Fa / UnderhillEventTable`.
