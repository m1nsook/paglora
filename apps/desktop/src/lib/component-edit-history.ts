/** Exact Markdown snapshots for the source-owned Components composer.
 * History never changes source formatting or serializes the visual parser.
 * A different current source invalidates old history instead of undoing edits
 * made in Write or Source. Memory is bounded for large local documents.
 */
export type ComponentEditHistory = {
  current: string;
  undo: string[];
  redo: string[];
};

export const MAX_COMPONENT_UNDO_STEPS = 32;
export const MAX_COMPONENT_HISTORY_CHARS = 2_000_000;

export function createComponentEditHistory(source: string): ComponentEditHistory {
  return { current: source, undo: [], redo: [] };
}

function bounded(snapshots: string[]): string[] {
  const kept = snapshots.slice(-MAX_COMPONENT_UNDO_STEPS);
  let chars = kept.reduce((total, source) => total + source.length, 0);
  while (kept.length && chars > MAX_COMPONENT_HISTORY_CHARS) {
    chars -= kept.shift()!.length;
  }
  return kept;
}

/** Record one successful component insertion as one undoable source edit. */
export function recordComponentEdit(
  history: ComponentEditHistory, before: string, after: string,
): ComponentEditHistory {
  const base = history.current === before ? history : createComponentEditHistory(before);
  if (before === after) return base;
  return { current: after, undo: bounded([...base.undo, before]), redo: [] };
}

/** Return null when there is nothing to undo or another editor changed source. */
export function undoComponentEdit(
  history: ComponentEditHistory, actualSource: string,
): ComponentEditHistory | null {
  if (history.current !== actualSource || !history.undo.length) return null;
  const previous = history.undo.at(-1)!;
  return {
    current: previous,
    undo: history.undo.slice(0, -1),
    redo: bounded([...history.redo, actualSource]),
  };
}

export function redoComponentEdit(
  history: ComponentEditHistory, actualSource: string,
): ComponentEditHistory | null {
  if (history.current !== actualSource || !history.redo.length) return null;
  const next = history.redo.at(-1)!;
  return {
    current: next,
    undo: bounded([...history.undo, actualSource]),
    redo: history.redo.slice(0, -1),
  };
}
