import { describe, it, expect } from 'vitest';
import { computeBulkShelfAssignments } from './bulkShelfAssignments';

describe('computeBulkShelfAssignments', () => {
  it('expands every selected shelf with the full book list', () => {
    const assignments = computeBulkShelfAssignments([7, 9], [1, 2, 3]);
    expect(assignments).toEqual([
      { shelfId: 7, bookIds: [1, 2, 3] },
      { shelfId: 9, bookIds: [1, 2, 3] },
    ]);
  });

  it('returns an empty list when no shelf is selected', () => {
    expect(computeBulkShelfAssignments([], [1, 2, 3])).toEqual([]);
  });

  it('returns an empty list when no books are selected', () => {
    expect(computeBulkShelfAssignments([7], [])).toEqual([]);
    expect(computeBulkShelfAssignments([], [])).toEqual([]);
  });

  it('copies the book id array so callers cannot mutate the input', () => {
    const bookIds = [1, 2];
    const [assignment] = computeBulkShelfAssignments([7], bookIds);
    assignment.bookIds.push(99);
    expect(bookIds).toEqual([1, 2]);
  });
});
