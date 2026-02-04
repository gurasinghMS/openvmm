// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

/**
 * PinRows manages a set of pinned row indices for fast lookup.
 * Uses a hash set (JavaScript Set) for O(1) access.
 */
export class PinRows {
  private pinned: Set<number>;

  private constructor() {
    this.pinned = new Set<number>();
  }

  /**
   * Creates a new PinRows instance with an empty set.
   */
  static new(): PinRows {
    return new PinRows();
  }

  /**
   * Adds an integer to the pinned set.
   * @param index - The row index to pin
   */
  add(index: number): void {
    this.pinned.add(index);
  }

  /**
   * Removes an integer from the pinned set.
   * @param index - The row index to unpin
   */
  remove(index: number): void {
    this.pinned.delete(index);
  }

  /**
   * Checks if an integer is in the pinned set.
   * @param index - The row index to check
   * @returns true if the row is pinned, false otherwise
   */
  contains(index: number): boolean {
    return this.pinned.has(index);
  }
}
