import { useMemo } from "react";
import { marked } from "marked";
import { renderPageContent } from "@/lib/site";
import { parseSourceComponents } from "@/lib/component-source-model";
import { Plus, Redo2, Undo2 } from "lucide-react";

/** The component canvas renders the real Markdown, not Tiptap's parsed tree.
 * Every insertion target belongs to the same token sequence used for saving.
 */
export function SourceComponentCanvas({
  source, selectedBoundary, onSelectBoundary, onEditText, canUndo, canRedo, onUndo, onRedo,
}: {
  source: string;
  selectedBoundary: number;
  onSelectBoundary: (boundary: number) => void;
  onEditText: () => void;
  canUndo: boolean;
  canRedo: boolean;
  onUndo: () => void;
  onRedo: () => void;
}) {
  const blocks = useMemo(() => parseSourceComponents(source).blocks.map(block => ({
    ...block,
    html: renderPageContent(
      marked.parse(block.raw, { async: false }) as string, "en",
    ).html,
  })), [source]);

  const slot = (boundary: number) => (
    <button key={"slot-" + boundary} type="button"
      data-component-position={boundary}
      aria-label={`Insert component at position ${boundary + 1}`}
      aria-pressed={selectedBoundary === boundary}
      onClick={() => onSelectBoundary(boundary)}
      className={`group relative flex h-7 w-full cursor-pointer items-center justify-center rounded text-xs transition-colors ${selectedBoundary === boundary ? "text-primary" : "text-muted-foreground hover:text-primary"}`}>
      <span className={`absolute inset-x-0 h-0.5 rounded bg-current transition-opacity ${selectedBoundary === boundary ? "opacity-80" : "opacity-0 group-hover:opacity-60"}`} />
      <span className={`relative z-10 flex size-5 items-center justify-center rounded-full border bg-background shadow-sm transition-opacity ${selectedBoundary === boundary ? "opacity-100" : "opacity-0 group-hover:opacity-100"}`}>
        <Plus className="size-3" />
      </span>
    </button>
  );

  return (
    <div data-component-drop-canvas
      className="mx-auto w-full max-w-3xl px-6 pb-28 pt-6 lg:px-10"
      aria-label="Documentation component composer">
      <div className="mb-5 flex items-center justify-between gap-3 rounded-lg border border-border bg-muted/40 px-4 py-3 text-xs text-muted-foreground">
        <span>Select an insertion line, then click or drag a component here.</span>
        <div className="flex shrink-0 items-center gap-1">
          <button type="button" onClick={onUndo} disabled={!canUndo}
            aria-label="Undo component change" title="Undo component change (Ctrl/Cmd+Z)"
            className="rounded-md p-2 text-foreground hover:bg-muted focus-visible:outline-2 focus-visible:outline-primary disabled:cursor-not-allowed disabled:opacity-40">
            <Undo2 className="size-4" aria-hidden="true" />
          </button>
          <button type="button" onClick={onRedo} disabled={!canRedo}
            aria-label="Redo component change" title="Redo component change (Ctrl/Cmd+Shift+Z)"
            className="rounded-md p-2 text-foreground hover:bg-muted focus-visible:outline-2 focus-visible:outline-primary disabled:cursor-not-allowed disabled:opacity-40">
            <Redo2 className="size-4" aria-hidden="true" />
          </button>
          <button type="button" className="ml-2 font-medium text-primary hover:underline"
            onClick={onEditText}>Write text</button>
        </div>
      </div>
      {slot(0)}
      {blocks.length === 0 && (
        <p className="py-14 text-center text-sm text-muted-foreground">Empty page. Choose a component from the right.</p>
      )}
      {blocks.map((block, index) => (
        <div key={"block-" + index}>
          <div role="button" tabIndex={0} data-component-block-index={index}
            aria-label={`Select Markdown block ${index + 1}`}
            onClick={() => onSelectBoundary(index + 1)}
            onDoubleClick={onEditText}
            onKeyDown={event => {
              if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onSelectBoundary(index + 1);
              }
            }}
            className="group block w-full cursor-pointer rounded-lg border border-transparent px-3 py-2 text-left transition-colors hover:border-primary/40 hover:bg-primary/5 focus-visible:border-primary focus-visible:outline-none"
          >
            <div className="paglora-component-preview pointer-events-none"
              dangerouslySetInnerHTML={{ __html: block.html }} />
          </div>
          {slot(index + 1)}
        </div>
      ))}
    </div>
  );
}
