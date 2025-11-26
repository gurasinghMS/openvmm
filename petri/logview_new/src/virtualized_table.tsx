// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

import React, {
  useState,
  useEffect,
  useRef,
  useLayoutEffect,
  useCallback,
  useMemo,
} from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  flexRender,
  type Row,
  type ColumnDef,
  type SortingState,
  useReactTable,
  getCoreRowModel,
  getSortedRowModel,
  getFilteredRowModel,
} from "@tanstack/react-table";
import "./styles/virtualized_table.css";

export interface VirtualizedTableProps<TData extends object> {
  data: TData[];
  columns: ColumnDef<TData, any>[];
  sorting: SortingState;
  onSortingChange: (
    updater: SortingState | ((old: SortingState) => SortingState)
  ) => void;
  columnWidthMap: Record<string, number>;
  estimatedRowHeight?: number; // default 50
  overscan?: number; // default 10
  /** Derive a className for a given row (virtual wrapper div). */
  getRowClassName?: (row: Row<TData>) => string;
  /** Handle row click events */
  onRowClick?: (row: Row<TData>, event: React.MouseEvent) => void;
  /** If provided, the virtualizer will scroll this row index into view (center aligned). */
  scrollToIndex?: number | null;
}

function defaultInferRowClass(row: Row<any>): string {
  const failed = row?.original?.metadata?.petriFailed;
  if (typeof failed === "number") {
    return failed > 0 ? "failed-row" : "passed-row";
  }
  return "passed-row";
}

export function VirtualizedTable<TData extends object>({
  data,
  columns,
  sorting,
  onSortingChange,
  columnWidthMap,
  estimatedRowHeight = 100,
  overscan = 20,
  getRowClassName,
  onRowClick,
  scrollToIndex,
}: VirtualizedTableProps<TData>): React.JSX.Element {
  const table = useReactTable({
    data,
    columns,
    state: {
      sorting,
    },
    onSortingChange,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
    getFilteredRowModel: getFilteredRowModel(),
    enableSorting: true,
    enableSortingRemoval: false,
    debugTable: false,
  });

  const { rows } = table.getRowModel();

  const visibleLeafColumns = table.getVisibleLeafColumns();
  const columnOrderById = useMemo(() => {
    const map = new Map<string, number>();
    visibleLeafColumns.forEach((col, index) => {
      map.set(col.id, index);
    });
    return map;
  }, [visibleLeafColumns]);

  const tableContainerRef = useRef<HTMLDivElement>(null);
  const headerWrapperRef = useRef<HTMLDivElement>(null);
  const [headerHeight, setHeaderHeight] = useState(25.5); // Initial estimate

  // Measure the header and set the value appropriately
  useLayoutEffect(() => {
    const el = headerWrapperRef.current;
    if (!el) return;
    setHeaderHeight(el.getBoundingClientRect().height);
  }, []);

  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => tableContainerRef.current,
    estimateSize: () => estimatedRowHeight,
    overscan,
    measureElement:
      typeof window !== "undefined" &&
      navigator.userAgent.indexOf("Firefox") === -1
        ? (element) => element?.getBoundingClientRect().height
        : undefined,
  });

  // Force recompute when data/rows change (e.g., during filtering/searching).
  // This ensures the virtualizer knows about new heights if the data changes.
  useEffect(() => {
    rowVirtualizer.calculateRange();
    rowVirtualizer.getVirtualItems().forEach((virtualRow) => {
      const el = document.querySelector(`[data-index="${virtualRow.index}"]`);
      if (el) {
        rowVirtualizer.measureElement(el);
      }
    });
  }, [rows.length, data, rowVirtualizer, sorting]);

  // Scroll to a requested index (center align) whenever scrollToIndex changes.
  useEffect(() => {
    if (scrollToIndex == null) return;
    if (scrollToIndex < 0 || scrollToIndex >= rows.length) return;
    try {
      rowVirtualizer.scrollToIndex(scrollToIndex, { align: "center" });
    } catch {
      /* no-op */
    }
  }, [scrollToIndex, rowVirtualizer, rows.length]);

  const handleCopy = useCallback(
    (event: React.ClipboardEvent<HTMLDivElement>) => {
      const selection = window.getSelection();
      if (!selection || selection.isCollapsed || selection.rangeCount === 0) {
        return;
      }

      const selectedCells = collectSelectedCells(selection);
      if (selectedCells.length === 0 || !event.clipboardData) {
        return;
      }

      const selectedRowIndices = new Set<number>();
      const selectedColumnIds = new Set<string>();
      const cellContent = new Map<
        number,
        Map<
          string,
          {
            html: string;
            text: string;
          }
        >
      >();

      for (const cell of selectedCells) {
        const rowIndexAttr = cell.getAttribute("data-row-index");
        const columnId = cell.getAttribute("data-column-id");
        if (rowIndexAttr == null || columnId == null) {
          continue;
        }

        const rowIndex = Number(rowIndexAttr);
        if (!Number.isFinite(rowIndex)) {
          continue;
        }

        selectedRowIndices.add(rowIndex);
        selectedColumnIds.add(columnId);

        const forRow = cellContent.get(rowIndex) ?? new Map();
        forRow.set(columnId, {
          html: cell.innerHTML,
          text: cell.innerText ?? cell.textContent ?? "",
        });
        cellContent.set(rowIndex, forRow);
      }

      if (selectedRowIndices.size === 0 || selectedColumnIds.size === 0) {
        return;
      }

      const sortedRows = Array.from(selectedRowIndices).sort((a, b) => a - b);
      const sortedColumnIds = Array.from(selectedColumnIds).sort((a, b) => {
        const left = columnOrderById.get(a) ?? Number.MAX_SAFE_INTEGER;
        const right = columnOrderById.get(b) ?? Number.MAX_SAFE_INTEGER;
        if (left === right) {
          return a.localeCompare(b);
        }
        return left - right;
      });

      const headerHtmlCells: string[] = [];
      const headerPlainCells: string[] = [];
      for (const columnId of sortedColumnIds) {
        const headerEl = Array.from(
          document.querySelectorAll<HTMLTableCellElement>(
            "th[data-column-id]"
          )
        ).find((candidate) => candidate.getAttribute("data-column-id") === columnId);
        const headerContent =
          headerEl?.querySelector<HTMLElement>(
            ".virtualized-table-header-content"
          ) ?? headerEl;
        const headerText = headerContent?.textContent?.trim();
        const finalHeaderText = headerText && headerText.length > 0
          ? headerText
          : columnId;
        headerHtmlCells.push(`<th>${escapeHtml(finalHeaderText)}</th>`);
        headerPlainCells.push(finalHeaderText);
      }

      const htmlBodyRows: string[] = [];
      const plainRows: string[] = [];

      for (const rowIndex of sortedRows) {
        const rowMap = cellContent.get(rowIndex);
        if (!rowMap) {
          continue;
        }

        const htmlCells: string[] = [];
        const plainCells: string[] = [];

        for (const columnId of sortedColumnIds) {
          const cellValue = rowMap.get(columnId);
          if (!cellValue) {
            htmlCells.push("<td></td>");
            plainCells.push("");
            continue;
          }

          htmlCells.push(`<td>${cellValue.html}</td>`);
          plainCells.push(cellValue.text.replace(/\s+/g, " ").trim());
        }

        htmlBodyRows.push(`<tr>${htmlCells.join("")}</tr>`);
        plainRows.push(plainCells.join("\t"));
      }

      if (htmlBodyRows.length === 0) {
        return;
      }

      event.preventDefault();

      const tableHtmlParts = ["<table>"];
      if (headerHtmlCells.length > 0) {
        tableHtmlParts.push(`<thead><tr>${headerHtmlCells.join("")}</tr></thead>`);
      }
      tableHtmlParts.push(`<tbody>${htmlBodyRows.join("")}</tbody></table>`);

      const htmlPayload = tableHtmlParts.join("");
      const plainPayload = [
        ...(headerHtmlCells.length > 0
          ? [headerPlainCells.join("\t")]
          : []),
        ...plainRows,
      ].join("\n");

      event.clipboardData.setData("text/html", htmlPayload);
      event.clipboardData.setData("text/plain", plainPayload);
    },
    [columnOrderById]
  );

  return (
    <div onCopy={handleCopy}>
      <div
        ref={headerWrapperRef}
        className="virtualized-table-header-container"
      >
        <table className="virtualized-table">
          <thead>
            {table.getHeaderGroups().map((headerGroup) => (
              <tr key={headerGroup.id}>
                {headerGroup.headers.map((header) => {
                  return (
                    <th
                      key={header.id}
                      data-column-id={header.column.id}
                      className={header.column.getCanSort() ? "sortable" : ""}
                      onClick={header.column.getToggleSortingHandler()}
                      style={{
                        width: columnWidthMap[header.column.id],
                      }}
                    >
                      <div className="virtualized-table-header-content">
                        {header.isPlaceholder
                          ? null
                          : flexRender(
                              header.column.columnDef.header,
                              header.getContext()
                            )}
                        {header.column.getCanSort() && (
                          <span className="sort-indicator">
                            {{
                              asc: "↑",
                              desc: "↓",
                            }[header.column.getIsSorted() as string] ?? "⇅"}
                          </span>
                        )}
                      </div>
                    </th>
                  );
                })}
              </tr>
            ))}
          </thead>
        </table>
      </div>
      <div
        ref={tableContainerRef}
        className="virtualized-table-body"
        style={{
          height: `calc(100vh - 3.2rem - ${headerHeight}px)`,
        }}
      >
        <div
          style={{
            height: `${rowVirtualizer.getTotalSize()}px`,
          }}
        >
          {rowVirtualizer.getVirtualItems().map((virtualRow) => {
            const row = rows[virtualRow.index] as Row<TData>;
            return (
              <div
                key={row.id}
                data-index={virtualRow.index}
                ref={rowVirtualizer.measureElement}
                className={`virtualized-table-row ${getRowClassName ? getRowClassName(row) : defaultInferRowClass(row)}`}
                style={{
                  position: "absolute",
                  width: "100%",
                  transform: `translateY(${virtualRow.start}px)`,
                }}
                onClick={
                  onRowClick ? (event) => onRowClick(row, event) : undefined
                }
              >
                <table className="virtualized-table">
                  <tbody>
                    <tr>
                      {row.getVisibleCells().map((cell) => {
                        return (
                          <td
                            key={cell.id}
                            data-row-index={virtualRow.index}
                            data-column-id={cell.column.id}
                            style={{
                              boxSizing: "border-box",
                              width: columnWidthMap[cell.column.id],
                            }}
                          >
                            {flexRender(
                              cell.column.columnDef.cell,
                              cell.getContext()
                            )}
                          </td>
                        );
                      })}
                    </tr>
                  </tbody>
                </table>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

function collectSelectedCells(selection: Selection): HTMLTableCellElement[] {
  const cells = new Set<HTMLTableCellElement>();
  for (let i = 0; i < selection.rangeCount; i += 1) {
    const range = selection.getRangeAt(i);
    collectCellsInRange(range, cells);
  }
  return Array.from(cells);
}

function collectCellsInRange(
  range: Range,
  target: Set<HTMLTableCellElement>
): void {
  const root = range.commonAncestorContainer;
  const walker = document.createTreeWalker(
    root,
    NodeFilter.SHOW_ELEMENT,
    {
      acceptNode: (node: Node) => {
        if (!(node instanceof HTMLElement)) {
          return NodeFilter.FILTER_SKIP;
        }
        if (!(node instanceof HTMLTableCellElement)) {
          return NodeFilter.FILTER_SKIP;
        }
        return range.intersectsNode(node)
          ? NodeFilter.FILTER_ACCEPT
          : NodeFilter.FILTER_SKIP;
      },
    }
  );

  const rootNode = walker.currentNode;
  if (
    rootNode instanceof HTMLTableCellElement &&
    range.intersectsNode(rootNode)
  ) {
    target.add(rootNode);
  }

  while (walker.nextNode()) {
    const node = walker.currentNode;
    if (node instanceof HTMLTableCellElement) {
      target.add(node);
    }
  }
}

function escapeHtml(input: string): string {
  return input
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}
