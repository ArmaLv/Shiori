/**
 * bulkShelfAssignments.ts
 *
 * Pure helper for the bulk "Add to Shelf…" flow (BulkShelfDialog):
 * expands (selected shelf × book ids) into the list of bulk assignments
 * to run via api.addBooksToShelf.
 */

export interface BulkShelfAssignment {
  shelfId: number;
  bookIds: number[];
}

export function computeBulkShelfAssignments(
  selectedShelfIds: number[],
  bookIds: number[],
): BulkShelfAssignment[] {
  if (bookIds.length === 0) return [];
  return selectedShelfIds.map((shelfId) => ({ shelfId, bookIds: [...bookIds] }));
}
