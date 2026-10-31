import { useCallback, useEffect, useLayoutEffect, useRef, useState, type FormEvent } from "react";
import type { ProjectDocument, ProjectSummary, ProjectVisual } from "@paglora/core";
import { validateProjectName } from "@paglora/core";
import { useTranslation } from "@/i18n";
import { EditorContent, useEditor } from "@tiptap/react";
import StarterKit from "@tiptap/starter-kit";
import { Markdown } from "@tiptap/markdown";
import { LocalImageNode } from "@/components/editor/local-image-node";
import { AssetLibrary } from "@/components/editor/asset-library";
import { InlinePageTitle } from "@/components/editor/inline-page-title";
import { ArrowLeft, BookOpen, Check, Columns2, LoaderCircle, Monitor, PanelLeftClose, PanelLeftOpen, PanelRightClose, PanelRightOpen, PencilLine, Plus, RotateCcw, SlidersHorizontal } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { PageNavigation, type PageAction } from "@/components/editor/page-navigation";
import { FormatToolbar } from "@/components/editor/format-toolbar";
import { BlockPalette } from "@/components/editor/block-palette";
import type { BlockKind } from "@/components/editor/block-catalog";
import { insertComponentAtBoundary, parseSourceComponents } from "@/lib/component-source-model";
import { createComponentEditHistory, recordComponentEdit, undoComponentEdit, redoComponentEdit } from "@/lib/component-edit-history";
import { SourceComponentCanvas } from "@/components/editor/source-component-canvas";
import { SiteBuilder } from "@/components/editor/site-builder";
import { SitePreview } from "@/components/editor/site-preview";
import { SiteDesigner } from "@/components/editor/site-designer";
import { ThemeSourceEditor } from "@/components/editor/theme-source-editor";
import { InlineSitePreview } from "@/components/editor/inline-site-preview";
import { StudioResizableSplit } from "@/components/editor/studio-resizable-split";
import { StudioNavigationLayout } from "@/components/editor/studio-navigation-layout";
import { STUDIO_SPLIT_LAYOUT_KEY } from "@/lib/studio-panel-layout";
import { editableCanvasText, findCanvasSourcePosition, replacePlainCanvasText, type CanvasBlockSelection } from "@/lib/canvas-blocks";
import { preservePagloraMarkers } from "@/lib/markdown-markers";
import { PageInspector } from "@/components/editor/page-inspector";
import { WorkspaceSidebar, type WorkspaceView } from "@/components/studio/workspace-sidebar";
import { StudioQuickFind } from "@/components/studio/studio-quick-find";
import { commandForEvent, DEFAULT_SHORTCUTS, readStudioShortcuts, shortcutLabel, STUDIO_SHORTCUTS_KEY, validateBinding, type ShortcutBindings, type StudioCommand } from "@/lib/studio-shortcuts";
import { StudioSurfaceMenu } from "@/components/studio/studio-surface-menu";
import { StudioTextContextMenu } from "@/components/studio/studio-text-context-menu";
import { StudioTitlebar } from "@/components/studio/studio-titlebar";
import { StudioTooltip } from "@/components/studio/studio-tooltip";
import { Toaster } from "@/components/ui/sonner";
import { toast } from "sonner";
import { Dashboard, Settings } from "@/components/studio/workspace-content";
import { createBackupTaskCoordinator, type BackupTask } from "@/lib/backup-task";
import { readableLocalPath } from "@/lib/windows-path";
import { StudioUpdateDialog, useStudioUpdater } from "@/components/studio/studio-updater";
import { StudioReleaseDialog, LAST_STUDIO_VERSION_KEY } from "@/components/studio/studio-release-dialog";
import { STUDIO_VERSION } from "@/lib/release-notes";
import { createProject, exportProjectBackup, importProjectBackup, isTauri, listProjects, listTrashedProjects, loadProject, migrateProjectStorage, projectStorageStatus, reconnectProjectStorage, returnToPreviousStorage, type ProjectStorageStatus, permanentlyDeleteTrashedProject, restoreTrashedProject, trashProject, updateProjectVisual, type TrashedProject } from "@/lib/projects";
import { listPages, loadPage, createPage, renamePage, duplicatePage, deletePage, reorderPages, savePage, setPageVisibility, setPageLayout, setPageNavigationGroup, type Page, type PageDocument } from "@/lib/pages";

function friendlyError(error: unknown) { return error instanceof Error ? error.message : String(error); }

function CreateProjectDialog({ open, onOpenChange, onCreate, disabled }: {
  open: boolean; onOpenChange: (value: boolean) => void;
  onCreate: (name: string) => Promise<void>; disabled: boolean;
}) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  useEffect(() => { if (!open) { setName(""); setError(""); setBusy(false); } }, [open]);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const validation = validateProjectName(name);
    if (validation) { setError(validation); return; }
    if (disabled) { setError(t("studio.projects.desktopRequired")); return; }
    setBusy(true); setError("");
    try { await onCreate(name.trim()); onOpenChange(false); }
    catch (cause) { setError(friendlyError(cause)); }
    finally { setBusy(false); }
  }
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <div className="mb-1 flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary"><BookOpen className="size-5"/></div>
          <DialogTitle>{t("studio.projects.createTitle")}</DialogTitle>
          <DialogDescription>{t("studio.projects.createDescription")}</DialogDescription>
        </DialogHeader>
        <form onSubmit={(e) => void submit(e)} className="mt-6 space-y-4">
          <div className="grid gap-2"><Label htmlFor="paglora-project-name">{t("studio.projects.projectName")}</Label><Input id="paglora-project-name" autoFocus placeholder={t("studio.projects.projectName")} value={name} onChange={(e) => setName(e.target.value)} maxLength={100} required /></div>
          {error && <p role="alert" className="rounded-lg bg-destructive/10 p-3 text-xs text-destructive">{error}</p>}
          <DialogFooter><Button type="button" variant="outline" onClick={() => onOpenChange(false)}>{t("studio.projects.cancel")}</Button><Button type="submit" disabled={busy || disabled}>{busy ? <LoaderCircle className="animate-spin"/> : <Plus/>} {t("studio.projects.create")}</Button></DialogFooter>
          {disabled && <p className="text-xs text-amber-700">{t("studio.projects.browserOnly")}</p>}
        </form>
      </DialogContent>
    </Dialog>
  );
}

function SinglePageEditor({ document, current, pages, onBack, onSaved, onChange, onModify, onPageMetadataSaved, findOpen, onFindOpenChange, findShortcut }: { document: ProjectDocument; current: PageDocument; pages: Page[]; onBack: () => void; onSaved: (project: ProjectSummary) => void; onChange: (id: string) => Promise<void>; onModify: (action: PageAction, id?: string, target?: string) => Promise<void>; onPageMetadataSaved: (page: Page) => void; findOpen: boolean; onFindOpenChange: (open: boolean) => void; findShortcut: string }) {
  const { t } = useTranslation();
  const latest = useRef(current.markdown);
  const componentHistory = useRef(createComponentEditHistory(current.markdown));
  const saveTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const queue = useRef<Promise<void>>(Promise.resolve());
  const revision = useRef(0);
  // Original Markdown stays authoritative when Tiptap parses an invalid tree.
  const rawSourceFallbackActive = useRef(false);
  const [saveState, setSaveState] = useState<"saved" | "unsaved" | "saving" | "error">("saved");
  const [saveError, setSaveError] = useState("");
  const [viewMode, setViewMode] = useState<"edit" | "split" | "site">("site");
  const [componentMode, setComponentMode] = useState<"compose" | "write">("compose");
  const [componentBoundary, setComponentBoundary] = useState(0);
  const [, refreshComponents] = useState(0);
  const [splitResetToken, setSplitResetToken] = useState(0);
  const [switchingView, setSwitchingView] = useState(false);
  const [siteRevision, setSiteRevision] = useState(0);
  const [showBlockTools, setShowBlockTools] = useState(true);
  const [showPages, setShowPages] = useState(true);
  const [showInspector, setShowInspector] = useState(true);
  const [selectedBlock, setSelectedBlock] = useState<CanvasBlockSelection | null>(null);
  const [pendingBlock, setPendingBlock] = useState<CanvasBlockSelection | null>(null);
  const [leaving, setLeaving] = useState(false);

  const persist = useCallback(async () => {
    if (saveTimer.current) clearTimeout(saveTimer.current);
    const content = latest.current;
    const capturedRevision = revision.current;
    setSaveState("saving");
    const run = queue.current.catch(() => undefined).then(() => savePage(document.project.id, current.page.id, content));
    queue.current = run.then(() => undefined, () => undefined);
    try {
      const updated = await run;
      onSaved(updated);
      setSiteRevision(value => value + 1);
      if (capturedRevision === revision.current) { setSaveState("saved"); setSaveError(""); }
    } catch (error) {
      setSaveState("error"); setSaveError(friendlyError(error));
      throw error;
    }
  }, [document.project.id, current.page.id, onSaved]);

  const editor = useEditor({
    extensions: [StarterKit, LocalImageNode, Markdown],
    content: current.markdown,
    contentType: "markdown",
    editorProps: { attributes: { class: "editor-content focus:outline-none" } },
    onUpdate: ({ editor: current }) => {
      if (rawSourceFallbackActive.current) {
        try {
          current.state.doc.check();
          rawSourceFallbackActive.current = false;
        } catch {
          setSaveError("The visual parser cannot safely save this page. Your original Markdown is preserved; edit it in Source.");
          return; // Never serialize malformed Tiptap content over valid .md.
        }
      }
      latest.current = preservePagloraMarkers(current.getMarkdown());
      // A Write-mode edit is not part of Components history. Never let Undo
      // silently roll back an unrelated text edit or a stale ProseMirror tree.
      componentHistory.current = createComponentEditHistory(latest.current);
      revision.current += 1;
      setSaveState("unsaved");
      if (saveTimer.current) clearTimeout(saveTimer.current);
      saveTimer.current = setTimeout(() => { void persist().catch(() => undefined); }, 850);
    },
  });

  /** Apply an exact source snapshot and let the existing serialized save queue
   * persist it. A broken Tiptap parse cannot overwrite this Markdown.
   */
  function applyComponentSource(nextSource: string, boundary: number) {
    latest.current = nextSource;
    revision.current++;
    rawSourceFallbackActive.current = true;
    setSaveState("unsaved");
    setSaveError("");
    if (saveTimer.current) clearTimeout(saveTimer.current);
    refreshComponents(n => n + 1);
    setComponentBoundary(Math.min(boundary, parseSourceComponents(nextSource).blocks.length));
    try {
      editor?.commands.setContent(nextSource, { contentType: "markdown", emitUpdate: false });
      editor?.state.doc.check();
      rawSourceFallbackActive.current = false;
    } catch { /* The original source remains authoritative. */ }
    void persist().catch(() => undefined);
  }

  function insertSourceComponent(kind: BlockKind, boundary: number): boolean {
    if (leaving || switchingView || viewMode === "site") return false;
    try {
      const before = latest.current;
      const nextSource = insertComponentAtBoundary(before, kind, boundary);
      componentHistory.current = recordComponentEdit(componentHistory.current, before, nextSource);
      applyComponentSource(nextSource, boundary + 1);
      return true;
    } catch (error) {
      const reason = friendlyError(error);
      toast.error("Could not insert component", { description: reason });
      setSaveError(reason);
      return false;
    }
  }

  function undoComponentInsertion(): boolean {
    if (leaving || switchingView || viewMode === "site") return false;
    const next = undoComponentEdit(componentHistory.current, latest.current);
    if (!next) return false;
    componentHistory.current = next;
    applyComponentSource(next.current, componentBoundary);
    return true;
  }

  function redoComponentInsertion(): boolean {
    if (leaving || switchingView || viewMode === "site") return false;
    const next = redoComponentEdit(componentHistory.current, latest.current);
    if (!next) return false;
    componentHistory.current = next;
    applyComponentSource(next.current, componentBoundary);
    return true;
  }

  // Source-backed component history is separate from Tiptap's text history.
  // Capture shortcuts only while Components is active and focus belongs to
  // this project, never inside Monaco, text fields or portal dialogs.
  useEffect(() => {
    if (componentMode !== "compose" || viewMode === "site") return;
    function componentHistoryShortcut(event: KeyboardEvent) {
      if (!(event.ctrlKey || event.metaKey) || event.altKey) return;
      const target = event.target;
      if (!(target instanceof Element) ||
          !target.closest("[data-paglora-project-root]") ||
          target.closest('input, textarea, select, [contenteditable], [role="textbox"], [data-paglora-source-editor], [data-slot="dialog-content"]')) return;
      const key = event.key.toLowerCase();
      const undo = key === "z" && !event.shiftKey;
      const redo = (key === "z" && event.shiftKey) || (key === "y" && !event.shiftKey);
      if (!undo && !redo) return;
      event.preventDefault();
      if (undo) undoComponentInsertion();
      else redoComponentInsertion();
    }
    window.addEventListener("keydown", componentHistoryShortcut);
    return () => window.removeEventListener("keydown", componentHistoryShortcut);
  }, [componentMode, viewMode, leaving, switchingView, editor, persist, componentBoundary]);

  // Desktop Ctrl+S saves the active Markdown file instead of opening WebView's
  // "Save page as..." dialog. The existing serialized autosave queue is reused.
  useEffect(() => {
    function saveShortcut(event: KeyboardEvent) {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "s") {
        // Monaco owns the Source modal's Ctrl+S. Do not save Markdown from a
        // different editor, or race against the custom CSS save.
        if (event.target instanceof Element && event.target.closest("[data-paglora-source-editor]")) return;
        event.preventDefault();
        void persist().catch(() => undefined);
      }
    }
    window.addEventListener("keydown", saveShortcut);
    return () => window.removeEventListener("keydown", saveShortcut);
  }, [persist]);

  // Studio starts in Canvas (read-only) but the SAME editor is used by
  // the formatting toolbar, block palette and autosave. Reassert editability
  // when switching views AND after React mounts its editable ProseMirror DOM.
  // A passive effect could leave commands pointing at a detached readonly
  // editor during the initial Canvas -> Edit layout transition.
  useLayoutEffect(() => {
    if (!editor || editor.isDestroyed) return;
    const editing = viewMode !== "site" && componentMode === "write";
    editor.setEditable(editing);
    if (!editing) return;
    const frame = requestAnimationFrame(() => {
      if (editor.isDestroyed) return;
      editor.setEditable(true);
      // Selecting Edit should put the typing caret into the real Tiptap
      // document, not leave focus on the view-switching button. Never steal
      // focus from a dialog or another active text input.
      const active = window.document.activeElement;
      if (editor.view.dom.isConnected && !(active instanceof Element &&
          active.closest('[data-slot="dialog-content"], [data-paglora-source-editor], input, textarea'))) {
        editor.commands.focus();
      }
    });
    return () => cancelAnimationFrame(frame);
  }, [editor, viewMode, componentMode]);
  useEffect(() => {
    if (viewMode !== "edit" || !pendingBlock || !editor) return;
    const position = pendingBlock.pageId === current.page.id
      ? findCanvasSourcePosition(editor, pendingBlock) : null;
    // Never edit an element based only on a potentially mismatched DOM index.
    // When a position is not confidently matched, safely open the page editor.
    if (position !== null) editor.commands.focus(position);
    else editor.commands.focus();
    setPendingBlock(null);
  }, [viewMode, pendingBlock, editor, current.page.id]);
  useEffect(() => () => { if (saveTimer.current) clearTimeout(saveTimer.current); }, []);

  async function runAfterSave(action: () => Promise<void>) {
    if (leaving) return;
    setLeaving(true);
    try { await persist(); await action(); }
    catch (cause) { setSaveState("error"); setSaveError(friendlyError(cause)); }
    finally { setLeaving(false); }
  }
  async function selectView(next: "edit" | "split" | "site") {
    if (next === viewMode || leaving || switchingView) return;
    if (next === "edit") {
      // Do not wait for the default Canvas mode's disabled editor effect.
      editor?.setEditable(true);
      setViewMode(next);
      return;
    }
    setSwitchingView(true);
    try {
      // Before revealing the public site, flush the Markdown save queue.
      // Later autosaves cause a fresh preview without replacing the editor.
      await persist();
      setViewMode(next);
    } catch (cause) {
      setSaveState("error");
      setSaveError(friendlyError(cause));
    } finally {
      setSwitchingView(false);
    }
  }
  async function renamePageInline(title: string) {
    if (leaving || switchingView) throw new Error("Studio is busy saving.");
    setLeaving(true);
    try {
      await persist();
      await onModify("rename", current.page.id, title);
      setSiteRevision(version => version + 1);
    } catch (cause) {
      setSaveState("error");
      setSaveError(friendlyError(cause));
      throw cause;
    } finally {
      setLeaving(false);
    }
  }

  function editCanvasBlock(block: CanvasBlockSelection) {
    setComponentMode("write");
    if (block.pageId !== current.page.id || leaving || switchingView) return;
    setSelectedBlock(block);
    setPendingBlock(block);
    setViewMode("edit");
  }

  async function back() { await runAfterSave(async () => onBack()); }
  const modify = (action: PageAction, id?: string, target?: string) => {
    void runAfterSave(() => onModify(action, id, target));
  };

  // Retain the TipTap editor DOM while using the website canvas to allow
  // source-mapped inline edits in site mode without losing editor state.
  const editorPane = (
    <div className="flex h-full min-h-0 min-w-0 flex-1 flex-col">
      <div className="flex shrink-0 items-center justify-between gap-2 border-b border-border bg-background px-4 py-2">
        <div className="flex items-center gap-1 rounded-md bg-muted p-1">
          <Button size="sm" type="button" variant={componentMode === "compose" ? "secondary" : "ghost"}
            onClick={() => setComponentMode("compose")}>Components</Button>
          <Button size="sm" type="button" variant={componentMode === "write" ? "secondary" : "ghost"}
            onClick={() => { setComponentMode("write"); editor?.commands.focus(); }}>Write</Button>
        </div>
        <span className="text-xs text-muted-foreground">Markdown · autosaved</span>
      </div>
      {saveError && <p role="alert" className="mx-5 mt-4 rounded-lg bg-destructive/10 p-3 text-sm text-destructive">
        {t("studio.editor.saveError", { message: saveError })}
      </p>}
      <section className="relative min-h-0 min-w-0 flex-1 overflow-y-auto"
        aria-label={t("studio.editor.documentationEditor")}>
        {componentMode === "compose" && (
          <SourceComponentCanvas source={latest.current}
            selectedBoundary={componentBoundary} onSelectBoundary={setComponentBoundary}
            canUndo={componentHistory.current.current === latest.current && componentHistory.current.undo.length > 0}
            canRedo={componentHistory.current.current === latest.current && componentHistory.current.redo.length > 0}
            onUndo={undoComponentInsertion} onRedo={redoComponentInsertion}
            onEditText={() => { setComponentMode("write"); editor?.commands.focus(); }} />
        )}
        <div className={componentMode === "write" ? "relative min-h-full" :
          "pointer-events-none absolute inset-0 -z-10 overflow-hidden opacity-0"} aria-hidden={componentMode !== "write"}>
          <FormatToolbar editor={editor} />
          <div className="mx-auto max-w-3xl px-6 py-9 lg:px-10 lg:py-12">
            <div onKeyDown={(event) => {
              if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "b" &&
                  event.target instanceof HTMLElement && event.target.isContentEditable) {
                event.stopPropagation();
              }
            }}>
              <EditorContent editor={editor} />
            </div>
          </div>
        </div>
      </section>
    </div>
  );
  const previewPane = (
    <InlineSitePreview
          projectId={document.project.id}
          page={current.page}
          pages={pages}
          savedVersion={siteRevision}
          onBeforeRefresh={persist}
          selectedBlock={selectedBlock}
          onBlockSelect={(block) => { setSelectedBlock(block); if (block) setShowInspector(true); }}
          onBlockEdit={editCanvasBlock}
          canInlineTextEdit={(block) => Boolean(editor && block.pageId === current.page.id && editableCanvasText(editor, block))}
          onApplyInlineText={(block, text) => {
            if (!editor || block.pageId !== current.page.id || leaving || switchingView) return false;
            return replacePlainCanvasText(editor, block, text);
          }}
          onNavigate={(pageId) => {
            if (pageId !== current.page.id) void runAfterSave(() => onChange(pageId));
          }}
        />
  );

  return (
    <main data-paglora-project-root className="flex h-full min-w-0 flex-1 flex-col bg-background">
      <header className="flex min-h-14 shrink-0 flex-wrap items-center justify-between gap-2 border-b border-border bg-card px-3 py-2 lg:px-5">
        <div className="flex min-w-0 items-center gap-2">
          <StudioTooltip label={showPages ? t("studio.editor.hidePages") : t("studio.editor.showPages")}><Button variant="ghost" size="icon-sm" aria-label={showPages ? t("studio.editor.hidePages") : t("studio.editor.showPages")} aria-pressed={showPages} onClick={() => setShowPages(value => !value)}>{showPages ? <PanelLeftClose /> : <PanelLeftOpen />}</Button></StudioTooltip>
          <Separator orientation="vertical" className="mx-1 h-5"/>
          <StudioTooltip label={t("studio.editor.backToProjects")}><Button onClick={() => void back()} variant="ghost" size="icon-sm" disabled={leaving} aria-label={t("studio.editor.backToProjects")}><ArrowLeft /></Button></StudioTooltip>
          <span className="hidden size-8 items-center justify-center rounded-lg bg-primary/10 text-primary sm:flex"><BookOpen className="size-4"/></span>
          <div className="min-w-0"><div className="max-w-44 truncate text-sm font-semibold lg:max-w-64">{document.project.name}</div><InlinePageTitle title={current.page.title} pageId={current.page.id} disabled={leaving || switchingView} onRename={renamePageInline} /></div>
        </div>
        <div className="flex items-center gap-2">
          <Badge variant={saveState === "error" ? "destructive" : "secondary"} className="gap-1.5 whitespace-nowrap">
            {saveState === "saving" ? <LoaderCircle className="size-3 animate-spin"/> : saveState === "saved" ? <Check className="size-3 text-emerald-600"/> : <span className="size-2 rounded-full bg-amber-400" />}
            {saveState === "saving" ? t("studio.editor.saving") : saveState === "saved" ? t("studio.editor.savedLocally") : saveState === "error" ? t("studio.editor.saveFailed") : t("studio.editor.unsaved")}
          </Badge>
          <Separator orientation="vertical" className="mx-1 hidden h-5 sm:block"/>
          {viewMode === "edit" && <StudioTooltip label={showBlockTools ? t("studio.editor.hideTools") : t("studio.editor.showTools")}><Button variant="ghost" size="icon-sm" aria-label={showBlockTools ? t("studio.editor.hideTools") : t("studio.editor.showTools")} aria-pressed={showBlockTools} onClick={() => setShowBlockTools(value => !value)}>{showBlockTools ? <PanelRightClose /> : <PanelRightOpen />}</Button></StudioTooltip>}
          {viewMode === "site" && <StudioTooltip label={showInspector ? t("studio.editor.hideProperties") : t("studio.editor.showProperties")}><Button variant="ghost" size="icon-sm" aria-label={showInspector ? t("studio.editor.hideProperties") : t("studio.editor.showProperties")} aria-pressed={showInspector} onClick={() => setShowInspector(value => !value)}><SlidersHorizontal /></Button></StudioTooltip>}
          <div role="group" aria-label={t("studio.editor.editorView")} className="inline-flex items-center gap-0.5 rounded-lg border border-border bg-muted/40 p-0.5">
            <Button type="button" size="sm" disabled={leaving || switchingView} variant={viewMode === "edit" ? "secondary" : "ghost"} aria-pressed={viewMode === "edit"} onClick={() => void selectView("edit")} className="gap-1.5 px-2.5"><PencilLine />{t("studio.editor.edit")}</Button>
            <Button type="button" size="sm" disabled={leaving || switchingView} variant={viewMode === "split" ? "secondary" : "ghost"} aria-pressed={viewMode === "split"} onClick={() => void selectView("split")} className="gap-1.5 px-2.5"><Columns2 />{t("studio.editor.split")}</Button>
            <Button type="button" size="sm" disabled={leaving || switchingView} variant={viewMode === "site" ? "secondary" : "ghost"} aria-pressed={viewMode === "site"} onClick={() => void selectView("site")} className="gap-1.5 px-2.5"><Monitor />{t("studio.editor.canvas")}</Button>
          </div>
          {viewMode === "split" && <Button type="button" size="sm" variant="ghost" className="gap-1.5"
            aria-label="Reset source and preview widths" onClick={() => {
              try { localStorage.removeItem(STUDIO_SPLIT_LAYOUT_KEY); } catch { /* Local preferences are optional. */ }
              setSplitResetToken(previous => previous + 1);
            }}>
            <RotateCcw className="size-3.5" /> Reset layout
          </Button>}
          <AssetLibrary projectId={document.project.id} canInsert={viewMode !== "site" && !leaving && !switchingView && Boolean(editor)}
            onBeforeCleanup={persist}
            onInsertImage={(src, alt) => Boolean(editor?.chain().focus().insertContent({ type: "pagloraImage", attrs: { src, alt } }).run())} />
          <ThemeSourceEditor projectId={document.project.id} currentPageId={current.page.id}
            previewPageId={current.page.isPublic ? current.page.id : "index"}
            pages={pages} onBeforePreview={persist} onPageMetadataSaved={onPageMetadataSaved} onPageSourceSaved={(pageId, markdown) => {
              if (pageId === current.page.id && editor) {
                // An explicit Monaco save is authoritative. Suppress Tiptap's
                // onUpdate so its old debounced autosave cannot overwrite it.
                if (saveTimer.current) clearTimeout(saveTimer.current);
                latest.current = markdown;
                componentHistory.current = createComponentEditHistory(markdown);
                revision.current += 1;
                refreshComponents(n => n + 1);
                setComponentBoundary(0);
                rawSourceFallbackActive.current = true;
                try {
                  editor.commands.setContent(markdown, { contentType: "markdown", emitUpdate: false });
                  editor.state.doc.check();
                  rawSourceFallbackActive.current = false;
                } catch {
                  // Source was already saved by Monaco. The visual parser must
                  // never turn a successful source edit into a rejected save.
                }
                setSaveState("saved");
                setSaveError("");
              }
              setSiteRevision(value => value + 1);
            }} />
          <SiteDesigner projectId={document.project.id} onBeforePreview={persist} />
          <SitePreview projectId={document.project.id} onBeforePreview={persist} />
          <SiteBuilder projectId={document.project.id} onBeforeBuild={persist} />
        </div>
      </header>
      <StudioNavigationLayout mode={viewMode}
        left={showPages ? <PageNavigation pages={pages} currentId={current.page.id} busy={leaving} onNavigate={(id) => { if (id !== current.page.id) void runAfterSave(() => onChange(id)); }} onAction={modify}/> : undefined}
        center={<StudioResizableSplit source={editorPane} preview={viewMode === "edit" ? null : previewPane}
          mode={viewMode} resetToken={splitResetToken} />}
        right={viewMode === "site" && showInspector ? <PageInspector page={current.page} busy={leaving || switchingView}
          selectedBlock={selectedBlock}
          allowTextEdit={Boolean(editor && selectedBlock && selectedBlock.pageId === current.page.id && editableCanvasText(editor, selectedBlock))}
          onSaveText={(nextText) => {
            if (!selectedBlock || selectedBlock.pageId !== current.page.id || !editor || leaving || switchingView) return false;
            if (!replacePlainCanvasText(editor, selectedBlock, nextText)) return false;
            setSelectedBlock(null);
            return true;
          }}
          onEditBlock={() => { if (selectedBlock) editCanvasBlock(selectedBlock); }}
          onClearBlock={() => setSelectedBlock(null)}
          onChange={(action, id, value) => modify(action, id, value)}
          onEditContent={() => void selectView("edit")} /> : viewMode === "edit" && showBlockTools ? <BlockPalette disabled={leaving || switchingView || componentMode !== "compose"}
          selectedBoundary={componentBoundary} onSelectedBoundaryChange={setComponentBoundary}
          onInsert={insertSourceComponent} /> : undefined}
      />
      <StudioQuickFind open={findOpen} onOpenChange={onFindOpenChange}
        projectId={document.project.id} shortcut={findShortcut} entries={pages.map(page => ({ id: page.id, title: page.title, description: page.navGroup, kind: "page" as const }))}
        onSelect={id => { if (id !== current.page.id) void runAfterSave(() => onChange(id)); }} />
    </main>
  );
}

function ProjectEditor(props: { document: ProjectDocument; onBack: () => void; onSaved: (project: ProjectSummary) => void; findOpen: boolean; onFindOpenChange: (open: boolean) => void; findShortcut: string }) {
  const { document } = props;
  const [pages, setPages] = useState<Page[]>([{id:"index",title:"Overview",isPublic:true,layout:"docs",navGroup:"Documentation"}]);
  const [current, setCurrent] = useState<PageDocument>({page:{id:"index",title:"Overview",isPublic:true,layout:"docs",navGroup:"Documentation"},markdown:document.markdown});
  const [failure, setFailure] = useState("");
  const id = document.project.id;
  useEffect(() => {
    let alive=true;
    void listPages(id).then(result => {
      if (!alive) return;
      setPages(result);
      // Hydrate the selected page's persisted title/layout/navigation metadata.
      // The initial Markdown comes from loadProject, but page metadata comes
      // from pages.json and must not remain a hard-coded placeholder.
      setCurrent(previous => {
        const loaded = result.find(page => page.id === previous.page.id);
        return loaded ? { ...previous, page: loaded } : previous;
      });
    }).catch(e => { if (alive) setFailure(friendlyError(e)); });
    return ()=>{alive=false;};
  },[id]);
  const change = async (pageId: string) => { setCurrent(await loadPage(id,pageId)); };
  const modify = async (action: PageAction, pageId?: string, target?: string) => {
    if(action==="create"){
      if(!target?.trim())return;
      setCurrent(await createPage(id,target.trim()));
    } else if(action==="rename" && pageId){
      if(!target?.trim())return;
      const updated=await renamePage(id,pageId,target.trim());
      if(current.page.id===pageId)setCurrent(p=>({...p,page:updated}));
    } else if(action==="duplicate" && pageId) setCurrent(await duplicatePage(id,pageId));
    else if(action==="visibility" && pageId && target){
      const updated = await setPageVisibility(id,pageId,target==="true");
      if(current.page.id===pageId)setCurrent(p=>({...p,page:updated}));
    }
    else if(action==="layout" && pageId && (target==="docs" || target==="wide" || target==="landing")){
      const updated = await setPageLayout(id,pageId,target);
      if(current.page.id===pageId)setCurrent(p=>({...p,page:updated}));
    }
    else if(action==="group" && pageId && target){
      const updated = await setPageNavigationGroup(id,pageId,target);
      if(current.page.id===pageId)setCurrent(p=>({...p,page:updated}));
    }
    else if(action==="delete" && pageId){
      await deletePage(id,pageId);
      if(current.page.id===pageId)setCurrent(await loadPage(id,"index"));
    } else if(action==="reorder" && pageId && target){
      const ids=pages.map(p=>p.id); const from=ids.indexOf(pageId),to=ids.indexOf(target);
      if(from<=0 || to<=0 || from===to)return;
      ids.splice(from,1);ids.splice(to,0,pageId);
      setPages(await reorderPages(id,ids));return;
    }
    setPages(await listPages(id));
  };
  return <>{failure && <div role="alert" className="fixed bottom-4 right-4 z-50 rounded-lg bg-red-50 p-3 text-red-600">{failure}</div>}
    <SinglePageEditor key={current.page.id} document={document} current={current} pages={pages}
      onBack={props.onBack} onSaved={props.onSaved} onChange={change} onModify={modify}
      onPageMetadataSaved={updated => {
        setPages(previous => previous.map(page => page.id === updated.id ? updated : page));
        setCurrent(previous => previous.page.id === updated.id ? { ...previous, page: updated } : previous);
      }}
      findOpen={props.findOpen} onFindOpenChange={props.onFindOpenChange} findShortcut={props.findShortcut}/></>;
}

export default function App() {
  const desktop = isTauri();
  const updater = useStudioUpdater(desktop);
  const [releaseNotesOpen, setReleaseNotesOpen] = useState(false);
  const [view, setView] = useState<WorkspaceView>("home");
  const [search, setSearch] = useState("");
  const [findOpen, setFindOpen] = useState(false);
  const [shortcuts, setShortcuts] = useState<ShortcutBindings>(readStudioShortcuts);
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [trashItems, setTrashItems] = useState<TrashedProject[]>([]);
  const [restoringTrashId, setRestoringTrashId] = useState<string | null>(null);
  const [deletingTrashId, setDeletingTrashId] = useState<string | null>(null);
  const [active, setActive] = useState<ProjectDocument | null>(null);
  const [creating, setCreating] = useState(false);
  const [loading, setLoading] = useState(desktop);
  const [error, setError] = useState("");
  const [location, setLocation] = useState("");
  const [storageMigrating, setStorageMigrating] = useState(false);
  const [storageError, setStorageError] = useState("");
  const [storageStatus, setStorageStatus] = useState<ProjectStorageStatus | null>(null);
  const [backupNotice, setBackupNotice] = useState("");
  const [backupPath, setBackupPath] = useState("");
  const [backupTask, setBackupTask] = useState<BackupTask | null>(null);
  // Synchronous lock: repeated clicks cannot start two archive operations before React paints.
  const backupCoordinator = useRef<ReturnType<typeof createBackupTaskCoordinator> | null>(null);
  if (!backupCoordinator.current) backupCoordinator.current = createBackupTaskCoordinator(setBackupTask);
  const { t } = useTranslation();

  // Only an actual installed version change opens the release dialog automatically.
  // Fresh installations record the first version without interrupting onboarding.
  useEffect(() => {
    if (!desktop || import.meta.env.DEV) return;
    try {
      const previous = localStorage.getItem(LAST_STUDIO_VERSION_KEY);
      if (previous && previous !== STUDIO_VERSION) setReleaseNotesOpen(true);
      else if (!previous) localStorage.setItem(LAST_STUDIO_VERSION_KEY, STUDIO_VERSION);
    } catch { /* Release notes remain manually accessible without localStorage. */ }
  }, [desktop]);

  const closeReleaseNotes = () => {
    try { localStorage.setItem(LAST_STUDIO_VERSION_KEY, STUDIO_VERSION); } catch {}
    setReleaseNotesOpen(false);
  };

  // Paglora is a desktop editor, not a web page. Prevent browser-only
  // print/save/reload/open shortcuts (and F5) in the Tauri WebView.
  // Do not block copy/paste/undo, or text formatting inside Tiptap.
  useEffect(() => {
    if (!desktop) return;
    function preventBrowserCommands(event: KeyboardEvent) {
      const command = event.ctrlKey || event.metaKey;
      const key = event.key.toLowerCase();
      if ((command && ["p", "s", "r", "o"].includes(key)) || event.key === "F5") {
        event.preventDefault();
      }
    }
    window.addEventListener("keydown", preventBrowserCommands, { capture: true });
    return () => window.removeEventListener("keydown", preventBrowserCommands, { capture: true });
  }, [desktop]);

  // Route customizable commands through one registry. Monaco keeps its
  // source find behavior; shortcut capture in Settings never triggers a command.
  useEffect(() => {
    function dispatchShortcut(event: KeyboardEvent) {
      const target = event.target;
      if (target instanceof Element && target.closest('[data-studio-shortcut-recorder], [data-paglora-source-editor]')) return;
      const command = commandForEvent(event, shortcuts);
      if (!command) {
        if (desktop && (event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "f") event.preventDefault();
        return;
      }
      if (target instanceof Element && target.closest('[data-slot="dialog-content"], [data-slot="alert-dialog-content"]')) return;
      event.preventDefault();
      event.stopPropagation();
      switch (command) {
        case "find": setFindOpen(true); break;
        case "new-project": if (!active) setCreating(true); break;
        case "projects": if (!active) setView("projects"); break;
        case "settings": if (!active) setView("settings"); break;
      }
    }
    window.addEventListener("keydown", dispatchShortcut, { capture: true });
    return () => window.removeEventListener("keydown", dispatchShortcut, { capture: true });
  }, [shortcuts, active, desktop]);

  const persistShortcuts = (next: ShortcutBindings): boolean => {
    try {
      localStorage.setItem(STUDIO_SHORTCUTS_KEY, JSON.stringify({ version: 1, bindings: next }));
      setShortcuts(next);
      return true;
    } catch { return false; }
  };

  const assignShortcut = (command: StudioCommand, stroke: string): string | null => {
    const invalid = validateBinding(command, stroke, shortcuts);
    if (invalid) return invalid;
    if (!persistShortcuts({ ...shortcuts, [command]: stroke })) return "Could not save shortcut preferences on this computer.";
    toast.success(t("studio.shortcuts.updated"), { description: t("studio.shortcuts.updatedDetail") });
    return null;
  };

  const resetShortcuts = () => {
    if (persistShortcuts({ ...DEFAULT_SHORTCUTS })) toast.success(t("studio.shortcuts.resetDone"));
    else toast.error(t("studio.shortcuts.saveError"));
  };

  const refresh = useCallback(async (strict = false) => {
    if (!desktop) return;
    setLoading(true);
    try {
      const [active, removed] = await Promise.all([listProjects(), listTrashedProjects()]);
      setProjects(active);
      setTrashItems(removed);
      setError("");
    }
    catch (cause) { setError(friendlyError(cause)); if (strict) throw cause; }
    finally { setLoading(false); }
  }, [desktop]);

  const inspectStorage = useCallback(async () => {
    if (!desktop) return;
    try {
      const next=await projectStorageStatus();
      setStorageStatus(next);
      setLocation(next.available ? next.configuredPath : "");
    } catch(error) { setStorageError(friendlyError(error)); }
  }, [desktop]);
  useEffect(() => { void refresh(); void inspectStorage(); }, [refresh, inspectStorage]);


  async function create(name: string) {
    const doc = await createProject(name);
    setProjects((prev) => [doc.project, ...prev]);
    setActive(doc);
    toast.success("Project created", { description: name.trim() });
  }
  async function open(id: string) {
    try { setError(""); setActive(await loadProject(id)); }
    catch (cause) { setError(friendlyError(cause)); }
  }
  async function exportBackup(id: string) {
    if (backupCoordinator.current?.active) return;
    try {
      const result = await backupCoordinator.current!.run(
        { kind: "export", phase: "writing", label: projects.find(project => project.id === id)?.name ?? "project" },
        async () => {
          setError(""); setBackupNotice(""); setBackupPath("");
          return exportProjectBackup(id);
        },
      );
      if (!result.started) return;
      const displayPath = readableLocalPath(result.value);
      setBackupPath(displayPath);
      setBackupNotice(t("studio.backup.saved", { path: displayPath }));
      toast.success("Project backup saved", { description: "A portable backup is ready in your local backups folder." });
    } catch (cause) {
      setError(friendlyError(cause));
      toast.error("Could not save project backup", { description: friendlyError(cause) });
    }
  }
  async function importBackup(file: File) {
    if (backupCoordinator.current?.active) return;
    if (!file.name.toLowerCase().endsWith(".paglora") || file.size === 0 || file.size > 96 * 1024 * 1024) {
      setError(t("studio.backup.invalid")); return;
    }
    let importedId: string | null = null;
    try {
      const result = await backupCoordinator.current!.run(
        { kind: "import", phase: "reading", label: file.name },
        async setPhase => {
          setError(""); setBackupNotice(""); setBackupPath("");
          const archive = Array.from(new Uint8Array(await file.arrayBuffer()));
          setPhase("restoring");
          importedId = await importProjectBackup(archive);
          setPhase("refreshing");
          // Do not report a fully refreshed workspace if the listing failed.
          await refresh(true);
          return importedId;
        },
      );
      if (!result.started) return;
      setBackupNotice(t("studio.backup.imported", { id: result.value }));
      toast.success("Project imported", { description: "The project is now available in your workspace." });
    } catch (cause) {
      const message = friendlyError(cause);
      if (importedId) {
        setBackupNotice(`Project ${importedId} was imported, but the workspace list could not refresh. Reopen Projects to retry.`);
        toast.error("Project imported; workspace refresh failed", { description: message });
      } else {
        setError(message);
        toast.error("Could not import project backup", { description: message });
      }
    }
  }
  async function removeProject(id: string) {
    if (backupCoordinator.current?.active) return;
    try {
      setError("");
      const moved = await trashProject(id);
      setProjects(previous => previous.filter(project => project.id !== id));
      try { setTrashItems(await listTrashedProjects()); } catch (cause) { console.warn("Unable to refresh recently removed projects", cause); }
      toast.success("Project moved to trash", {
        description: "Files are preserved in Paglora's local trash folder.",
        action: { label: "Copy path", onClick: () => {
          if (navigator.clipboard?.writeText) void navigator.clipboard.writeText(moved);
        } },
      });
    } catch (cause) {
      const message = friendlyError(cause);
      setError(message);
      toast.error("Could not remove project", { description: message });
    }
  }
  async function restoreProject(trashId: string) {
    if (restoringTrashId) return;
    setRestoringTrashId(trashId);
    try {
      const restored = await restoreTrashedProject(trashId);
      await refresh();
      toast.success("Project restored", { description: `${restored.name} is available in your workspace.` });
    } catch (cause) {
      const message = friendlyError(cause);
      toast.error("Unable to restore project", { description: message });
    } finally {
      setRestoringTrashId(null);
    }
  }
  async function deleteTrashedProject(trashId: string, confirmation: string) {
    if (restoringTrashId || deletingTrashId) return;
    setDeletingTrashId(trashId);
    try {
      await permanentlyDeleteTrashedProject(trashId, confirmation);
      await refresh();
      toast.success("Project permanently deleted", { description: "It cannot be restored." });
    } catch (cause) {
      toast.error("Could not delete project", { description: friendlyError(cause) });
    } finally { setDeletingTrashId(null); }
  }
  async function customizeProject(id: string, visual: ProjectVisual) {
    try {
      const updated = await updateProjectVisual(id, visual);
      setProjects(previous => previous.map(project => project.id === id ? updated : project));
      toast.success("Project appearance saved", { description: "Your category, accent and icon are included in project backups." });
    } catch (cause) {
      const message = friendlyError(cause);
      toast.error("Couldn't update project appearance", { description: message });
      throw cause;
    }
  }

  async function moveStorage(destination: string) {
    if (storageMigrating || active) return;
    setStorageMigrating(true);
    setStorageError("");
    try {
      const next = await migrateProjectStorage(destination);
      setLocation(next);
      await inspectStorage();
      await refresh();
      toast.success(t("studio.storage.migrated"), {
        description: t("studio.storage.migratedDetail"),
      });
    } catch (cause) {
      const message = friendlyError(cause);
      setStorageError(message);
      toast.error(t("studio.storage.migrationError"), { description: message });
      throw cause;
    } finally {
      setStorageMigrating(false);
    }
  }
  async function reconnectStorage(destination: string, confirmation: string) {
    if (storageMigrating || active) return;
    setStorageMigrating(true);
    setStorageError("");
    try {
      const next=await reconnectProjectStorage(destination,confirmation);
      setLocation(next);
      await inspectStorage();
      await refresh();
      toast.success(t("studio.storage.reconnected"), { description: t("studio.storage.reconnectedDetail") });
    } catch(error) {
      const message=friendlyError(error);
      setStorageError(message);
      toast.error(t("studio.storage.reconnectError"), { description:message });
      throw error;
    } finally {setStorageMigrating(false);}
  }
  async function returnStorage(confirmation: string) {
    if (storageMigrating || active) return;
    setStorageMigrating(true);
    setStorageError("");
    try {
      const next=await returnToPreviousStorage(confirmation);
      setLocation(next);
      await inspectStorage();
      await refresh();
      toast.success(t("studio.storage.returned"), { description: t("studio.storage.returnedDetail") });
    } catch(error) {
      const message=friendlyError(error);
      setStorageError(message);
      toast.error(t("studio.storage.returnError"), { description: message });
      throw error;
    } finally {setStorageMigrating(false);}
  }
  async function checkStorageAgain() {
    await inspectStorage();
    await refresh();
  }
  const onSaved = useCallback((updated: ProjectSummary) => {
    setProjects((previous) => previous.map((p) => p.id === updated.id ? updated : p));
  }, []);
  function navigate(next: WorkspaceView) { if (active || storageMigrating) return; setView(next); }

  return (
    <div data-paglora-desktop-shell={desktop ? "" : undefined} className="flex h-svh min-h-0 w-full flex-col overflow-hidden bg-background">
      {desktop && <StudioTitlebar activeProject={active?.project.name} />}
      <SidebarProvider className="min-h-0 min-w-0 flex-1 overflow-hidden" defaultOpen>
      {!active && <WorkspaceSidebar view={view} onNavigate={navigate} editing={false}/>}
      <SidebarInset className="h-full min-w-0 overflow-hidden">
        {active ? <ProjectEditor key={active.project.id} document={active} onBack={() => { setActive(null); setFindOpen(false); void refresh(); }} onSaved={onSaved} findOpen={findOpen} onFindOpenChange={setFindOpen} findShortcut={shortcutLabel(shortcuts.find)}/> : view === "settings" ? <Settings desktop={desktop} location={location} storageStatus={storageStatus} onCheckStorage={() => void checkStorageAgain()} onReconnectStorage={reconnectStorage} onReturnStorage={returnStorage} updater={updater} onOpenReleaseNotes={() => setReleaseNotesOpen(true)} shortcuts={shortcuts} onAssignShortcut={assignShortcut} onResetShortcuts={resetShortcuts} onMigrateStorage={moveStorage} storageMigrating={storageMigrating} storageError={storageError}/> : <Dashboard projects={projects} trashItems={trashItems} restoringTrashId={restoringTrashId || deletingTrashId} onRestoreProject={(trashId) => void restoreProject(trashId)} onDeleteTrashedProject={(trashId, confirmation) => void deleteTrashedProject(trashId, confirmation)} onUpdateVisual={customizeProject} view={view} loading={loading} error={error} desktop={desktop} search={search} setSearch={setSearch} openCreate={() => setCreating(true)} openProject={(id) => void open(id)} onExportBackup={(id) => void exportBackup(id)} onImportBackup={(file) => void importBackup(file)} onTrashProject={(id) => void removeProject(id)} backupNotice={backupNotice} backupPath={backupPath} backupTask={backupTask}/>}
      </SidebarInset>
      {!active && <StudioQuickFind open={findOpen} onOpenChange={setFindOpen} shortcut={shortcutLabel(shortcuts.find)}
        entries={projects.map(project => ({ id: project.id, title: project.name, description: "Local documentation project", kind: "project" as const }))}
        onSelect={id => { void open(id); }} />}
      <StudioTextContextMenu />
      <StudioSurfaceMenu editing={Boolean(active)} findShortcut={shortcutLabel(shortcuts.find)}
        onFind={() => setFindOpen(true)}
        onHome={() => navigate("home")}
        onSettings={() => navigate("settings")} />
      <Toaster position="top-center" richColors closeButton visibleToasts={3} duration={5000} />
      <CreateProjectDialog open={creating} onOpenChange={setCreating} onCreate={create} disabled={!desktop}/>
      <StudioUpdateDialog updater={updater}/>
      <StudioReleaseDialog open={releaseNotesOpen} onClose={closeReleaseNotes}/>
      </SidebarProvider>
    </div>
  );
}
