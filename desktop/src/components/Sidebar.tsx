import { Archive, Disc3, Import, Music4, Piano, Tag as TagIcon } from "lucide-react";
import type { AssetKind, KindCounts, Source, Tag } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";

/**
 * Where the list is looking. A scene and a tone are different things to go looking for, so the
 * kind is a destination rather than a filter over a mixed list: the sidebar picks one, and
 * sources and tags narrow within it. A file scope still exists for a single imported file,
 * though nothing in the sidebar points at one now that sources are a flat list.
 */
export type Scope =
  | { view: "library" }
  | { view: "source"; id: number }
  | { view: "file"; id: number; sourceId: number }
  | { view: "songs" };

export function Sidebar({
  scope,
  onScope,
  kind,
  onKind,
  counts,
  sources,
  songCount,
  tags,
  activeTags,
  onToggleTag,
  onImport,
}: {
  scope: Scope;
  onScope: (scope: Scope) => void;
  kind: AssetKind;
  onKind: (kind: AssetKind) => void;
  /** Scenes and tones in the current scope, so the counts follow a source or a search. */
  counts: KindCounts;
  sources: Source[];
  songCount: number;
  tags: Tag[];
  activeTags: string[];
  onToggleTag: (tag: string) => void;
  onImport: () => void;
}) {
  return (
    <div className="flex h-full w-60 min-w-[12rem] flex-col text-sidebar-foreground">
      <div className="scroll-region flex-1">
        <nav className="flex flex-col gap-5 p-3 pt-4">
          <div className="flex flex-col gap-0.5">
            <Row
              icon={Disc3}
              iconClassName="text-scene"
              label="Scenes"
              count={counts.scenes}
              active={scope.view !== "songs" && kind === "scene"}
              onClick={() => onKind("scene")}
            />
            <Row
              icon={Piano}
              iconClassName="text-tone"
              label="Tones"
              count={counts.tones}
              active={scope.view !== "songs" && kind === "tone"}
              onClick={() => onKind("tone")}
            />
            <Row
              icon={Music4}
              label="Songs"
              count={songCount}
              active={scope.view === "songs"}
              onClick={() => onScope({ view: "songs" })}
            />
          </div>

          <Section title={`Sources (${sources.length})`}>
            {sources.length === 0 ? (
              <p className="px-2 py-1 text-xs text-muted-foreground">Nothing imported yet.</p>
            ) : (
              sources.map((source) => (
                <Row
                  key={source.id}
                  icon={source.archived_at ? Archive : Import}
                  label={source.name}
                  count={source.asset_count}
                  active={scope.view === "source" && scope.id === source.id}
                  dim={Boolean(source.archived_at)}
                  onClick={() => onScope({ view: "source", id: source.id })}
                />
              ))
            )}
          </Section>

          {tags.length > 0 && (
            <Section title="Tags">
              <div className="flex flex-wrap gap-1 px-1">
                {tags.map((tag) => (
                  <button
                    key={tag.name}
                    type="button"
                    onClick={() => onToggleTag(tag.name)}
                    className={cn(
                      "inline-flex min-h-6 items-center gap-1 rounded-md border px-2 py-1 text-xs transition-colors",
                      activeTags.includes(tag.name)
                        ? "border-transparent bg-primary text-primary-foreground"
                        : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                    )}
                  >
                    <TagIcon className="size-3" />
                    {tag.name}
                    <span className="tabular-nums">{tag.count}</span>
                  </button>
                ))}
              </div>
            </Section>
          )}
        </nav>
      </div>

      <div className="p-3">
        <Button className="w-full" onClick={onImport}>
          <Import data-icon="inline-start" />
          Import
        </Button>
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-1">
      <h2 className="px-2 text-[11px] font-medium tracking-wider text-muted-foreground uppercase">
        {title}
      </h2>
      {children}
    </div>
  );
}

function Row({
  icon: Icon,
  iconClassName,
  label,
  count,
  active,
  onClick,
  dim,
}: {
  icon: React.ComponentType<{ className?: string }>;
  /** Scenes and tones carry their hue here too, so the nav matches the rows it leads to. */
  iconClassName?: string;
  label: string;
  count: number;
  active: boolean;
  onClick: () => void;
  dim?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={label}
      aria-current={active ? "true" : undefined}
      className={cn(
        // Selection is the fill alone, as in the asset list.
        "flex w-full min-w-0 items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors",
        active
          ? "bg-sidebar-accent text-sidebar-accent-foreground"
          : "text-muted-foreground hover:bg-sidebar-accent/60 hover:text-sidebar-accent-foreground",
        dim && "opacity-50",
      )}
    >
      <Icon className={cn("size-4 shrink-0", iconClassName)} />
      <span className="truncate">{label}</span>
      <span className="ml-auto shrink-0 text-xs tabular-nums">{count}</span>
    </button>
  );
}



