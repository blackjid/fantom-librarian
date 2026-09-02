import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, Settings2 } from "lucide-react";
import {
  api,
  message,
  onMenu,
  type Asset,
  type ImportReport,
  type AssetKind,
  type Facets,
  type KindCounts,
  type Origin,
  type Plays,
  type Query,
  type Song,
  type Source,
  type Tag,
  type WorkspaceInfo,
} from "@/lib/api";
import { Welcome } from "@/components/Welcome";
import { Sidebar, type Scope } from "@/components/Sidebar";
import { AssetList, NoSelection } from "@/components/AssetList";
import { AssetDetail } from "@/components/AssetDetail";
import { SongsPanel } from "@/components/SongsPanel";
import { Resizer } from "@/components/Resizer";
import { ImportDialog } from "@/components/ImportDialog";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Spinner } from "@/components/ui/spinner";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { fileLabel, plural } from "@/lib/format";

/** Pane widths in px: where each starts, and how far it may go either way. */
const SIDEBAR = { initial: 240, min: 192, max: 320 };
const LIST = { initial: 416, min: 288, max: 544 };

export default function App() {
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [resuming, setResuming] = useState(true);
  const [slowResume, setSlowResume] = useState(false);

  const [scope, setScope] = useState<Scope>({ view: "library" });
  const [search, setSearch] = useState("");
  const [kind, setKind] = useState<AssetKind>("scene");
  const [activeTags, setActiveTags] = useState<string[]>([]);
  const [engines, setEngines] = useState<string[]>([]);
  const [models, setModels] = useState<string[]>([]);
  const [origin, setOrigin] = useState<Origin | null>(null);
  const [plays, setPlays] = useState<Plays | null>(null);

  const [assets, setAssets] = useState<Asset[]>([]);
  const [counts, setCounts] = useState<KindCounts>({ scenes: 0, tones: 0 });
  const [sources, setSources] = useState<Source[]>([]);
  const [songs, setSongs] = useState<Song[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [facets, setFacets] = useState<Facets>({
    engines: [],
    models: [],
    origins: [],
    plays: [],
  });
  const [selected, setSelected] = useState<Asset | null>(null);
  const [selectedSong, setSelectedSong] = useState<number | null>(null);

  const searchRef = useRef<HTMLInputElement>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [sidebarWidth, setSidebarWidth] = useState(SIDEBAR.initial);
  const [listWidth, setListWidth] = useState(LIST.initial);
  /** Width the detail pane has had to absorb because the list was already at a bound. */
  const spill = useRef(0);
  const [importing, setImporting] = useState(false);
  const [report, setReport] = useState<ImportReport | null>(null);

  const pickWorkspace = useCallback(async () => {
    const picked = await open({ directory: true, multiple: false, title: "Open a library" });
    if (typeof picked !== "string") return;
    try {
      setWorkspace(await api.openWorkspace(picked, !(await api.isWorkspace(picked))));
      setSelected(null);
      setScope({ view: "library" });
    } catch (e) {
      setError(message(e));
    }
  }, []);

  const closeWorkspace = useCallback(async () => {
    await api.closeWorkspace();
    setWorkspace(null);
    setSelected(null);
    setAssets([]);
  }, []);

  // Menu items do nothing of their own; they reach the same handlers the window uses.
  useEffect(() => {
    const unlisten = onMenu((action) => {
      switch (action) {
        case "import":
          setImporting(true);
          break;
        case "open-library":
          void pickWorkspace();
          break;
        case "close-library":
          void closeWorkspace();
          break;
        case "reveal-library":
          if (workspace) void revealItemInDir(workspace.path);
          break;
        case "find":
          setScope((current) => (current.view === "songs" ? { view: "library" } : current));
          searchRef.current?.focus();
          searchRef.current?.select();
          break;
      }
    });
    return () => {
      void unlisten.then((off) => off());
    };
  }, [pickWorkspace, closeWorkspace, workspace?.path]);

  const spread = useCallback((delta: number) => {
    setListWidth((current) => {
      let remaining = delta;
      // What the detail last had to absorb comes back to it first, so growing the window
      // retraces the steps shrinking it took.
      if (spill.current !== 0 && Math.sign(spill.current) !== Math.sign(remaining)) {
        const repaid = Math.min(Math.abs(remaining), Math.abs(spill.current)) * Math.sign(remaining);
        spill.current += repaid;
        remaining -= repaid;
      }
      const room = remaining > 0 ? LIST.max - current : LIST.min - current;
      const taken = remaining > 0 ? Math.min(remaining, room) : Math.max(remaining, room);
      spill.current += remaining - taken;
      return current + taken;
    });
  }, []);

  // The sidebar keeps the width it was given; the window's is the list's to take, up to its
  // bounds, and the detail pane takes what is left over.
  useEffect(() => {
    let previous = window.innerWidth;
    const onResize = () => {
      const delta = window.innerWidth - previous;
      previous = window.innerWidth;
      if (delta !== 0) spread(delta);
    };
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, [spread]);

  // Reopen last session's library, so the app lands where it was left.
  useEffect(() => {
    const slow = setTimeout(() => setSlowResume(true), 150);
    api
      .resumeWorkspace()
      .then(setWorkspace)
      .catch(() => undefined)
      .finally(() => {
        clearTimeout(slow);
        setResuming(false);
      });
    return () => clearTimeout(slow);
  }, []);

  const reloadSidebar = useCallback(async () => {
    if (!workspace) return;
    try {
      const [nextSources, nextTags, nextSongs, stats] = await Promise.all([
        api.listSources(),
        api.listTags(),
        api.listSongs(),
        api.getStats(),
      ]);
      setSources(nextSources);
      setTags(nextTags);
      setSongs(nextSongs);
      setWorkspace((current) => (current ? { ...current, stats } : current));
    } catch (e) {
      setError(message(e));
    }
  }, [workspace?.path]);

  useEffect(() => {
    void reloadSidebar();
  }, [reloadSidebar]);

  /** The scope and search, without the kind — the sidebar counts need both sides of it. */
  const baseQuery: Query = useMemo(
    () => ({
      search,
      source_id: scope.view === "source" ? scope.id : null,
      file_id: scope.view === "file" ? scope.id : null,
      tags: activeTags,
      engines,
      models,
      origin,
      plays,
    }),
    [search, scope, activeTags.join(" "), engines.join(" "), models.join(" "), origin, plays],
  );

  const reloadAssets = useCallback(async () => {
    if (!workspace || scope.view === "songs") return;
    setLoading(true);
    try {
      const [rows, totals, available] = await Promise.all([
        api.listAssets({ ...baseQuery, kind }),
        api.countAssets(baseQuery),
        api.listFacets({ ...baseQuery, kind }),
      ]);
      setAssets(rows);
      setCounts(totals);
      setFacets(available);
      setError(null);
    } catch (e) {
      setError(message(e));
    } finally {
      setLoading(false);
    }
  }, [workspace?.path, scope.view, baseQuery, kind]);

  // Typing should not fire a query per keystroke over a library of thousands.
  useEffect(() => {
    const timer = setTimeout(() => void reloadAssets(), search ? 180 : 0);
    return () => clearTimeout(timer);
  }, [reloadAssets, search]);

  // Keep the open detail pane in step with an edit made inside it.
  const refreshSelected = useCallback(async () => {
    if (!selected) return;
    try {
      setSelected(await api.getAsset(selected.id));
    } catch {
      setSelected(null);
    }
  }, [selected?.id]);

  const onChanged = useCallback(async () => {
    await Promise.all([refreshSelected(), reloadAssets(), reloadSidebar()]);
  }, [refreshSelected, reloadAssets, reloadSidebar]);

  /** What the list header calls the current scope, and the path beneath it when there is one. */
  const heading = useMemo(() => {
    // A source or a file is still only ever showing one kind, so the subtitle says which.
    const kindLabel = kind === "scene" ? "Scenes" : "Tones";
    if (scope.view === "source") {
      const source = sources.find((s) => s.id === scope.id);
      return { title: source?.name ?? "Source", subtitle: kindLabel };
    }
    if (scope.view === "file") {
      const file = sources.flatMap((s) => s.files).find((f) => f.id === scope.id);
      const name = file?.file_name ?? "File";
      return {
        title: fileLabel(name),
        subtitle: name.includes("/") ? `${kindLabel} · ${name}` : kindLabel,
      };
    }
    return { title: kindLabel, subtitle: undefined };
  }, [scope, sources, kind]);

  // Reopening is a local database read and usually beats the eye. Showing the spinner only once
  // it is genuinely slow keeps a flash of one off every launch.
  if (resuming) {
    return slowResume ? (
      <div className="flex h-screen items-center justify-center">
        <Spinner />
      </div>
    ) : (
      <div className="h-screen" />
    );
  }

  if (!workspace) {
    return <Welcome onOpen={setWorkspace} />;
  }

  return (
    <div className="flex h-screen flex-col overflow-hidden">
      {/* `deep`, not a bare attribute: bare drags only on the header element itself, so a drag
          starting on a child span does nothing. Clickable children still block the drag. */}
      <header
        data-tauri-drag-region="deep"
        className="flex h-11 shrink-0 items-center gap-3 pr-3 pl-20"
      >
        <span className="text-sm font-medium">{workspace.name}</span>
        <span className="truncate text-xs text-muted-foreground" title={workspace.path}>
          {workspace.path}
        </span>
        <div className="ml-auto flex items-center gap-2">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="icon-sm" aria-label="Library menu">
                <Settings2 />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuGroup>
                <DropdownMenuItem onSelect={() => void revealItemInDir(workspace.path)}>
                  <FolderOpen />
                  Reveal library folder
                </DropdownMenuItem>
                <DropdownMenuItem onSelect={() => void closeWorkspace()}>
                  Close library
                </DropdownMenuItem>
              </DropdownMenuGroup>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </header>

      {report && <ImportSummary report={report} onDismiss={() => setReport(null)} />}
      {error && (
        <Alert variant="destructive" className="m-3 w-auto">
          <AlertTitle>Something went wrong</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      {/* Panes shrink to their floors, then the shell scrolls. The detail pane is the only
          `flex-1`, so without a floor it is the one that collapses to nothing. */}
      <div className="scroll-region-x flex min-h-0 flex-1 overflow-x-auto pr-2 pb-1.5">
        <Sidebar
          width={sidebarWidth}
          scope={scope}
          onScope={setScope}
          kind={kind}
          onKind={(next) => {
            setKind(next);
            // Picking a kind is a move to the top of that side of the library, not a filter
            // laid over wherever you happened to be. A scene's models are not a tone's, so the
            // facets go with it.
            setScope({ view: "library" });
            setSelected(null);
            setEngines([]);
            setModels([]);
            setOrigin(null);
            setPlays(null);
          }}
          counts={counts}
          sources={sources}
          songCount={songs.length}
          tags={tags}
          activeTags={activeTags}
          facets={facets}
          engines={engines}
          models={models}
          origin={origin}
          plays={plays}
          onToggleEngine={(value) =>
            setEngines((current) =>
              current.includes(value) ? current.filter((e) => e !== value) : [...current, value],
            )
          }
          onToggleModel={(value) =>
            setModels((current) =>
              current.includes(value) ? current.filter((m) => m !== value) : [...current, value],
            )
          }
          onOrigin={(next) => setOrigin((current) => (current === next ? null : next))}
          onPlays={(next) => setPlays((current) => (current === next ? null : next))}
          onToggleTag={(tag) =>
            setActiveTags((current) =>
              current.includes(tag) ? current.filter((t) => t !== tag) : [...current, tag],
            )
          }
          onImport={() => setImporting(true)}
        />

        <Resizer
          label="Sidebar width"
          width={sidebarWidth}
          onWidth={setSidebarWidth}
          onReset={() => setSidebarWidth(SIDEBAR.initial)}
          min={SIDEBAR.min}
          max={SIDEBAR.max}
        />

        {scope.view === "songs" ? (
          <SongsPanel
            songs={songs}
            selectedId={selectedSong}
            onSelect={(song) => setSelectedSong(song?.id ?? null)}
            onChanged={reloadSidebar}
            linkTarget={selected}
          />
        ) : (
          // One panel holds both: the list is the panel's own surface, and the detail is a card
          // inset within it — contained by the list rather than butted against it.
          <div className="surface flex min-h-0 min-w-0 flex-1 overflow-hidden rounded-2xl bg-panel">
            <AssetList
              width={listWidth}
              assets={assets}
              loading={loading}
              selectedId={selected?.id ?? null}
              onSelect={setSelected}
              search={search}
              onSearch={setSearch}
              activeTags={activeTags}
              onClearTag={(tag) => setActiveTags((c) => c.filter((t) => t !== tag))}
              title={heading.title}
              subtitle={heading.subtitle}
              searchRef={searchRef}
            />

            {/* The strip sits in the card's own margin, so dragging it costs no extra gap. */}
            <Resizer
              label="List width"
              width={listWidth}
              onWidth={(next) => {
                spill.current = 0;
                setListWidth(next);
              }}
              onReset={() => {
                spill.current = 0;
                setListWidth(LIST.initial);
              }}
              min={LIST.min}
              max={LIST.max}
              className="my-2"
            />

            <div className="surface my-2 mr-2 min-w-[22rem] flex-1 overflow-hidden rounded-lg bg-panel-raised">
              {selected ? <AssetDetail asset={selected} onChanged={onChanged} /> : <NoSelection />}
            </div>
          </div>
        )}
      </div>

      <footer className="flex h-6 shrink-0 items-center justify-end gap-1 pr-3 pl-3 text-[11px] tabular-nums text-muted-foreground">
        <span>{plural(workspace.stats.scenes, "scene")}</span>
        <span>·</span>
        <span>{plural(workspace.stats.tones, "tone")}</span>
        <span>·</span>
        <span>{plural(workspace.stats.samples, "sample")}</span>
      </footer>

      <ImportDialog
        open={importing}
        onOpenChange={setImporting}
        onImported={async (next) => {
          setReport(next);
          setScope({ view: "source", id: next.source_id });
          await reloadSidebar();
        }}
      />
    </div>
  );
}

/** What an import actually did — including what it declined to do. */
function ImportSummary({ report, onDismiss }: { report: ImportReport; onDismiss: () => void }) {
  const nothingNew =
    report.scenes_added === 0 && report.tones_added === 0 && report.samples_catalogued === 0;

  return (
    <Alert className="m-3 w-auto">
      <AlertTitle>Imported “{report.source_name}”</AlertTitle>
      <AlertDescription>
        <div className="flex flex-col gap-2">
          <p>
            {plural(report.files_imported, "file")} · {plural(report.scenes_added, "new scene")} ·{" "}
            {plural(report.tones_added, "new tone")} ·{" "}
            {plural(report.samples_catalogued, "sample")}
            {report.assets_consolidated > 0 &&
              ` · ${report.assets_consolidated} matched material already in your library`}
            {report.files_skipped > 0 && ` · ${report.files_skipped} already imported`}
            {report.files_invalid > 0 && ` · ${plural(report.files_invalid, "file")} unusable`}
          </p>
          {nothingNew && report.assets_consolidated > 0 && (
            <p className="text-muted-foreground">
              Everything here was already in your library; the new source now shares it.
            </p>
          )}
          {report.warnings.length > 0 && (
            <details>
              <summary className="cursor-pointer text-xs">
                {plural(report.warnings.length, "note")} about this import
              </summary>
              <ul className="mt-1 flex flex-col gap-0.5 font-mono text-xs" data-selectable>
                {report.warnings.map((warning) => (
                  <li key={warning}>{warning}</li>
                ))}
              </ul>
            </details>
          )}
          <Button size="sm" variant="outline" className="self-start" onClick={onDismiss}>
            Dismiss
          </Button>
        </div>
      </AlertDescription>
    </Alert>
  );
}
