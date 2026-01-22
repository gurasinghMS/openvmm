// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

import { ColumnDef } from "@tanstack/react-table";
import { LogEntry } from "../data_defs";

export const defaultSorting = [
  { id: "relative", desc: false }, // Sort by status ascending, failed tests first
];

export const columnWidthMap = {
  relative: 110,
  severity: 80,
  source: 100,
  screenshot: 100,
  pin: 24,
};

export function createColumns(
  setModalContent: (content: string | null) => void,
  pinnedIndices: Set<number>,
): ColumnDef<LogEntry>[] {
  return [
    {
      id: "pin",
      header: "",
      cell: (info) => {
        const isPinned = pinnedIndices.has(info.row.original.index);
        if (!isPinned) return null;
        return (
          <svg
            viewBox="0 0 24 24"
            width="14"
            height="14"
            fill="currentColor"
            style={{ opacity: 0.7 }}
            aria-label="Pinned"
          >
            <path d="M16,12V4H17V2H7V4H8V12L6,14V16H11.2V22H12.8V16H18V14L16,12Z" />
          </svg>
        );
      },
      enableSorting: false,
    },
    {
      accessorKey: "relative",
      header: "Timestamp",
      cell: (info) => (
        <span title={info.row.original.timestamp}>
          {info.getValue() as string}
        </span>
      ),
      enableSorting: true,
    },
    {
      accessorKey: "severity",
      header: "Severity",
      enableSorting: false,
    },
    {
      accessorKey: "source",
      header: "Source",
      enableSorting: false,
    },
    {
      id: "message",
      accessorFn: (row) => row.logMessage, // Use text for sorting/filtering
      header: "Message",
      cell: (info) => (
        <>
          <div>{info.row.original.logMessage.message}</div>
          {info.row.original.logMessage.links?.map((link, idx) => (
            <a
              key={idx}
              href={link.url}
              className="attachment"
              target="_blank"
              rel="noopener noreferrer"
              data-inspect={link.inspect}
              style={{ marginLeft: 8 }}
            >
              {link.text}
            </a>
          ))}
        </>
      ),
      enableSorting: false, // Sorting by full message text is not useful
    },
    {
      id: "screenshot",
      header: "Screenshot",
      cell: (info) => {
        const screenshot = info.row.original.screenshot;
        return screenshot ? (
          <img
            src={screenshot}
            alt="Screenshot"
            style={{
              maxWidth: "100px",
              maxHeight: "50px",
              cursor: "pointer",
              objectFit: "contain",
            }}
            onClick={(e) => {
              e.stopPropagation();
              setModalContent(screenshot);
            }}
          />
        ) : (
          ""
        );
      },
      enableSorting: false,
    },
  ];
}
