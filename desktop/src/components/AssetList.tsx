import { useEffect, useMemo, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Disc3, Piano, Search, X } from "lucide-react";
import type { Asset, AssetSource } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Skeleton } from "@/components/ui/skeleton";
import { fileLabel, plural } from "@/lib/format";

export type KindFilter = "all" | "scene" | "tone";

/**
 * The middle pane: everything in the current scope, searchable across both kinds at once —
 * searching "Rhodes" should reach a standalone tone and the scenes that use it.
 *
 * The kind filter lives here rather than in the sidebar because it applies to every scope: the
 * whole library, one source, or one file inside it.
 */
export function AssetList({
  assets,
  loading,
  selectedId,
  onSelect,
  search,
  onSearch,
  kind,
  onKind,
  counts,
  activeTags,
  onClearTag,
  title,
  subtitle,
}: {
  assets: Asset[];
  loading: boolean;
  selectedId: number | null;
  onSelect: (asset: Asset) => void;
  search: string;
  onSearch: (value: string) => void;
  kind: KindFilter;
  onKind: (kind: KindFilter) => void;
  counts: { scenes: number; tones: number };
  activeTags: string[];
  onClearTag: (tag: string) => void;
  title: string;
  subtitle?: string;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  // The same pack loaded into two instruments yields scenes that are byte-different only in which
  // user-tone slot they point at — same name, same everything else. They stay separate items, so
  // the rows have to say which is which rather than repeat one name five times.
  const ambiguous = useMemo(() => {
    const seen = new Set<string>();
    const repeated = new Set<string>();
    for (const asset of assets) {
      const key = `${asset.kind}:${asset.fantom_name}`;
      if (seen.has(key)) repeated.add(key);
      seen.add(key);
    }
    return repeated;
  }, [assets]);

  // A whole backup is a couple of thousand sounds. Only the visible slice is in the DOM, which is
  // what keeps this list scrolling at the same speed however big the library gets.
  const virtual = useVirtualizer({
    count: assets.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 56,
    overscan: 12,
    getItemKey: (index) => assets[index]?.id ?? index,
  });

  // Keep the selected row on screen when the arrows walk it past either edge.
  useEffect(() => {
    if (selectedId === null) return;
    const index = assets.findIndex((asset) => asset.id === selectedId);
    if (index >= 0) virtual.scrollToIndex(index, { align: "auto" });
    // `virtual` is recreated on every render; keying on the id alone is what makes this fire once.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedId]);

  /** Arrow keys walk the list; Home and End jump to its ends. */
  function onKeyDown(event: React.KeyboardEvent) {
    const step = { ArrowDown: 1, ArrowUp: -1, PageDown: 10, PageUp: -10 }[event.key];
    const index = assets.findIndex((asset) => asset.id === selectedId);

    let next: number;
    if (step !== undefined) {
      next = index < 0 ? (step > 0 ? 0 : assets.length - 1) : index + step;
    } else if (event.key === "Home") {
      next = 0;
    } else if (event.key === "End") {
      next = assets.length - 1;
    } else {
      return;
    }

    event.preventDefault();
    const target = assets[Math.min(Math.max(next, 0), assets.length - 1)];
    if (target) onSelect(target);
  }

  return (
    <div className="flex h-full w-[26rem] shrink-0 flex-col border-r">
      <div className="flex flex-col gap-2 border-b p-3">
        <div className="flex items-baseline justify-between gap-2">
          <div className="flex min-w-0 flex-col">
            <h2 className="truncate text-sm font-medium" title={subtitle ?? title}>
              {title}
            </h2>
            {subtitle && (
              <span className="truncate text-xs text-muted-foreground" title={subtitle}>
                {subtitle}
              </span>
            )}
          </div>
          <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
            {loading ? "…" : plural(assets.length, "item")}
          </span>
        </div>

        <div className="relative">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            value={search}
            onChange={(e) => onSearch(e.target.value)}
            placeholder="Search names and notes…"
            className="pl-8"
            // Down out of the search field lands on the list without a mouse.
            onKeyDown={(e) => {
              if (e.key === "ArrowDown") {
                e.preventDefault();
                listRef.current?.focus();
                if (selectedId === null && assets[0]) onSelect(assets[0]);
              }
            }}
          />
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <ToggleGroup
            type="single"
            size="sm"
            value={kind}
            onValueChange={(value) => value && onKind(value as KindFilter)}
          >
            <ToggleGroupItem value="all">
              All
              <span className="ml-1 text-[10px] tabular-nums opacity-60">
                {counts.scenes + counts.tones}
              </span>
            </ToggleGroupItem>
            <ToggleGroupItem value="scene">
              Scenes
              <span className="ml-1 text-[10px] tabular-nums opacity-60">{counts.scenes}</span>
            </ToggleGroupItem>
            <ToggleGroupItem value="tone">
              Tones
              <span className="ml-1 text-[10px] tabular-nums opacity-60">{counts.tones}</span>
            </ToggleGroupItem>
          </ToggleGroup>

          {activeTags.map((tag) => (
            <Badge key={tag} variant="secondary" className="gap-1">
              {tag}
              <button type="button" onClick={() => onClearTag(tag)} aria-label={`Clear ${tag}`}>
                <X className="size-3" />
              </button>
            </Badge>
          ))}
        </div>
      </div>

      <div ref={scrollRef} className="scroll-region flex-1">
        {loading ? (
          <div className="flex flex-col gap-2 p-3">
            {Array.from({ length: 8 }).map((_, i) => (
              <Skeleton key={i} className="h-12 w-full" />
            ))}
          </div>
        ) : assets.length === 0 ? (
          <Empty className="py-16">
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <Search />
              </EmptyMedia>
              <EmptyTitle>Nothing here</EmptyTitle>
              <EmptyDescription>
                {search || activeTags.length > 0
                  ? "No scene or tone matches this search."
                  : "Import a pack or a backup to fill your library."}
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        ) : (
          <ul
            ref={listRef}
            tabIndex={0}
            role="listbox"
            aria-label="Library"
            aria-activedescendant={selectedId ? `asset-${selectedId}` : undefined}
            onKeyDown={onKeyDown}
            style={{ height: virtual.getTotalSize() }}
            className="relative p-1.5 outline-none"
          >
            {virtual.getVirtualItems().map((item) => {
              const asset = assets[item.index];
              if (!asset) return null;
              return (
                <Row
                  key={item.key}
                  ref={virtual.measureElement}
                  index={item.index}
                  offset={item.start}
                  asset={asset}
                  selected={asset.id === selectedId}
                  ambiguous={ambiguous.has(`${asset.kind}:${asset.fantom_name}`)}
                  onSelect={() => {
                    // Focus belongs to the list, not the row: one focus ring, and the arrows
                    // keep working after a click.
                    listRef.current?.focus();
                    onSelect(asset);
                  }}
                />
              );
            })}
          </ul>
        )}
      </div>
    </div>
  );
}

function Row({
  ref,
  index,
  offset,
  asset,
  selected,
  ambiguous,
  onSelect,
}: {
  /** The virtualizer measures each row, so rows may be any height. */
  ref: (element: HTMLElement | null) => void;
  index: number;
  offset: number;
  asset: Asset;
  selected: boolean;
  /** Another item in this list carries the same name, so the row must distinguish itself. */
  ambiguous: boolean;
  onSelect: () => void;
}) {
  const isScene = asset.kind === "scene";
  const Icon = isScene ? Disc3 : Piano;
  const summary =
    asset.detail.kind === "scene"
      ? `${asset.detail.bpm.toFixed(2)} BPM · ${plural(asset.detail.active_zones, "zone")}`
      : `${asset.detail.engine}${asset.detail.area ? ` · ${asset.detail.area}` : ""}`;

  return (
    <li
      ref={ref}
      id={`asset-${asset.id}`}
      data-index={index}
      role="option"
      aria-selected={selected}
      className="absolute top-0 left-0 w-full px-1.5"
      style={{ transform: `translateY(${offset}px)` }}
    >
      <button
        type="button"
        onClick={onSelect}
        tabIndex={-1}
        className={cn(
          // The kind rail is the only colour on a resting row; selection adds a background.
          "flex w-full items-start gap-2.5 rounded-md border-l-2 px-2 py-2 text-left transition-colors outline-none",
          selected ? "border-l-current bg-accent" : "border-l-transparent hover:bg-accent/50",
          isScene ? "text-scene" : "text-tone",
          asset.archived_at && "opacity-50",
        )}
      >
        <Icon className="mt-0.5 size-4 shrink-0" />
        <div className="flex min-w-0 flex-1 flex-col gap-0.5 text-foreground">
          <div className="flex items-baseline gap-2">
            <span className="truncate text-sm font-medium">{asset.fantom_name}</span>
            {asset.sources.length > 1 && (
              <Badge variant="outline" className="shrink-0 text-[10px]">
                {asset.sources.length}×
              </Badge>
            )}
          </div>
          <span className="truncate text-xs text-muted-foreground">
            {summary}
            {ambiguous && asset.sources[0] && (
              <>
                {" · "}
                <span className="opacity-80">{origin(asset.sources[0])}</span>
              </>
            )}
          </span>
          {asset.tags.length > 0 && (
            <div className="flex flex-wrap gap-1 pt-0.5">
              {asset.tags.map((tag) => (
                <Badge key={tag} variant="secondary" className="text-[10px]">
                  {tag}
                </Badge>
              ))}
            </div>
          )}
        </div>
        <span className="shrink-0 pt-0.5 text-[10px] tracking-wide uppercase opacity-70">
          {asset.kind}
        </span>
      </button>
    </li>
  );
}

/**
 * Where a row came from, precisely enough to tell it from its namesakes.
 *
 * The same pack loaded into two instruments produces scenes identical but for the tone slot they
 * point at — and one backup can hold two of them. So the file alone is not always enough; the
 * scene number settles it.
 */
function origin(source: AssetSource): string {
  const file = fileLabel(source.file_name);
  return source.area === "PRFa" ? `${file} #${source.slot}` : file;
}

/** Nothing selected yet — the detail pane's resting state. */
export function NoSelection() {
  return (
    <Empty className="h-full">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <Disc3 />
        </EmptyMedia>
        <EmptyTitle>Nothing selected</EmptyTitle>
        <EmptyDescription>
          Pick a scene or a tone to see its zones, where it came from, and what it needs.
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}
