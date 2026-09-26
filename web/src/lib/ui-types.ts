/** Small shapes shared between components. */

/** A problem on an interview step: the id of the field to focus, and what to fix. */
export interface FieldProblem {
  id: string;
  message: string;
}

/** One row of a scenario's with/without comparison. */
export interface ComparisonRow {
  bucket: string;
  with: string;
  without: string;
}
