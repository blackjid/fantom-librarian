import { useEffect, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Disc3, Search, X } from "lucide-react";
import type { Asset } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { Skeleton } from "@/components/ui/skeleton";
import { plural } from "@/lib/format";

/** What the user actually narrowed by, so the empty state can name it back to them. */
function describeFilter(search: string, tags: string[]): string {
  const parts = [];
  if (search) parts.push(`“${search}”`);
  if (tags.length > 0) parts.push(tags.map((t) => `#${t}`).join(" "));
  return parts.join(" + ");
}

/**
 * The middle pane: one kind of material at a time, in the current scope. The sidebar chooses
 * between scenes and tones, so this list never mixes them and every row means the same thing.
 */
export function AssetList({
  assets,
  loading,
  selectedId,
  onSelect,
  search,
  onSearch,
  activeTags,
  onClearTag,
  title,
  subtitle,
  searchRef,
}: {
  assets: Asset[];
  loading: boolean;
  selectedId: number | null;
  onSelect: (asset: Asset) => void;
  search: string;
  onSearch: (value: string) => void;
  activeTags: string[];
  onClearTag: (tag: string) => void;
  title: string;
  subtitle?: string;
  /** Lets the Find menu item put the caret in the search box. */
  searchRef?: React.RefObject<HTMLInputElement | null>;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const [focused, setFocused] = useState(false);
  const filtered = Boolean(search) || activeTags.length > 0;
  const searching = focused || Boolean(search);

  // A whole backup is a couple of thousand sounds. Only the visible slice is in the DOM, which is
  // what keeps this list scrolling at the same speed however big the library gets.
  const virtual = useVirtualizer({
    count: assets.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => 30,
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
    // The negative margin runs the panel under the detail pane's left edge, so the list reads as
    // one surface the detail is resting on rather than two panes butted together.
    <div className="-mr-3 flex h-full w-[26rem] min-w-[20rem] flex-col overflow-hidden rounded-l-xl bg-panel">
      <div className="flex flex-col p-3">
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

        {/* The field is not part of the resting list: it is here for Find and for as long as a
            search stands, then it folds away. Collapsed, not unmounted — Find focuses it. */}
        <div className={cn("relative", searching ? "mt-2" : "h-0 overflow-hidden opacity-0")}>
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            ref={searchRef}
            value={search}
            onChange={(e) => onSearch(e.target.value)}
            onFocus={() => setFocused(true)}
            onBlur={() => setFocused(false)}
            placeholder="Search names and notes…"
            className="pl-8"
            onKeyDown={(e) => {
              // Down out of the search field lands on the list without a mouse.
              if (e.key === "ArrowDown") {
                e.preventDefault();
                listRef.current?.focus();
                if (selectedId === null && assets[0]) onSelect(assets[0]);
              }
              // Escape drops the search and the field with it.
              if (e.key === "Escape") {
                onSearch("");
                e.currentTarget.blur();
              }
            }}
          />
        </div>

        {/* Only the tag chips live here now; the kind is a place in the sidebar, not a filter. */}
        {activeTags.length > 0 && (
          <div className="mt-2 flex flex-wrap items-center gap-2">
            {activeTags.map((tag) => (
              <Badge key={tag} variant="secondary" className="gap-1">
                {tag}
                {/* The chip stays small; only the target grows to 24×24. */}
                <button
                  type="button"
                  onClick={() => onClearTag(tag)}
                  aria-label={`Clear ${tag}`}
                  className="relative rounded-full after:absolute after:-inset-[6px] after:content-['']"
                >
                  <X className="size-3" />
                </button>
              </Badge>
            ))}
          </div>
        )}
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
              <EmptyTitle>
                {filtered ? `No results for ${describeFilter(search, activeTags)}` : "Nothing here"}
              </EmptyTitle>
              <EmptyDescription>
                {filtered
                  ? "No scene or tone in this scope matches."
                  : "Import a pack or a backup to fill your library."}
              </EmptyDescription>
            </EmptyHeader>
            {/* An empty result the user cannot leave is a dead end: the way out goes with it. */}
            {filtered && (
              <EmptyContent>
                <Button
                  size="sm"
                  variant="outline"
                  onClick={() => {
                    onSearch("");
                    for (const tag of activeTags) onClearTag(tag);
                  }}
                >
                  Clear search and filters
                </Button>
              </EmptyContent>
            )}
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
            // `focus`, not `focus-visible`: the list takes focus programmatically, from the search
            // field's ArrowDown and from a row click, and `:focus-visible` matches neither.
            className="relative rounded-md p-1.5 outline-none focus:ring-2 focus:ring-ring"
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
  onSelect,
}: {
  /** The virtualizer measures each row, so rows may be any height. */
  ref: (element: HTMLElement | null) => void;
  index: number;
  offset: number;
  asset: Asset;
  selected: boolean;
  onSelect: () => void;
}) {
  // One fact per row: how big a scene is, what engine a tone runs on. Everything else about an
  // item — its sources, its slot, its tempo — is a click away in the detail pane.
  const summary =
    asset.detail.kind === "scene"
      ? plural(asset.detail.active_zones, "zone")
      : asset.detail.engine;

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
          // A row carries no kind mark of its own: the list is one kind at a time, and the
          // sidebar already says which. Selection is a background, nothing more.
          "flex w-full items-center gap-2 rounded-md px-2 py-1 text-left transition-colors outline-none",
          selected ? "bg-accent" : "hover:bg-accent/50",
          asset.archived_at && "opacity-50",
        )}
      >
        <span className="truncate text-sm font-medium text-foreground">{asset.fantom_name}</span>

        {asset.tags.map((tag) => (
          <Badge key={tag} variant="secondary" className="shrink-0 text-[10px]">
            {tag}
          </Badge>
        ))}

        {/* Meta sits in its own right-hand column so the names keep one clean left edge. */}
        <span className="ml-auto shrink-0 truncate text-xs text-muted-foreground">{summary}</span>
      </button>
    </li>
  );
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
