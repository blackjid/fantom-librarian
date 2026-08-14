import { useState } from "react";
import {
  Archive,
  ChevronRight,
  Database,
  FileMusic,
  Import,
  Library,
  Music4,
  Package,
  Tag as TagIcon,
  TriangleAlert,
  Waves,
} from "lucide-react";
import type { LibraryFile, Role, Source, Stats, Tag } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { fileLabel } from "@/lib/format";

/**
 * Where the list is looking. The sidebar answers only this — *which* material — while the kind
 * filter above the list answers *what sort*. Splitting them that way stops the two controls
 * offering the same three choices.
 */
export type Scope =
  | { view: "library" }
  | { view: "source"; id: number }
  | { view: "file"; id: number; sourceId: number }
  | { view: "songs" };

/** Roles get an icon apiece, because "a backup" and "a scene export" behave nothing alike. */
const ROLE_ICON: Record<Role, React.ComponentType<{ className?: string }>> = {
  backup: Database,
  "scene-bank": FileMusic,
  "tone-bank": Package,
  "sample-bank": Waves,
  unknown: FileMusic,
};

export function Sidebar({
  scope,
  onScope,
  stats,
  sources,
  songCount,
  tags,
  activeTags,
  onToggleTag,
  onImport,
}: {
  scope: Scope;
  onScope: (scope: Scope) => void;
  stats: Stats;
  sources: Source[];
  songCount: number;
  tags: Tag[];
  activeTags: string[];
  onToggleTag: (tag: string) => void;
  onImport: () => void;
}) {
  // A source opens when it is the thing being browsed, and stays open once opened by hand.
  const [opened, setOpened] = useState<Set<number>>(new Set());

  const isOpen = (id: number) =>
    opened.has(id) ||
    (scope.view === "source" && scope.id === id) ||
    (scope.view === "file" && scope.sourceId === id);

  const toggle = (id: number) =>
    setOpened((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  return (
    <div className="flex h-full w-60 shrink-0 flex-col border-r bg-sidebar text-sidebar-foreground">
      <div className="scroll-region flex-1">
        <nav className="flex flex-col gap-5 p-3 pt-4">
          <div className="flex flex-col gap-0.5">
            <Row
              icon={Library}
              label="All sounds"
              count={stats.scenes + stats.tones}
              active={scope.view === "library"}
              onClick={() => onScope({ view: "library" })}
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
                <div key={source.id} className="flex flex-col">
                  <Row
                    icon={source.archived_at ? Archive : Import}
                    label={source.name}
                    count={source.asset_count}
                    active={scope.view === "source" && scope.id === source.id}
                    dim={Boolean(source.archived_at)}
                    onClick={() => onScope({ view: "source", id: source.id })}
                    disclosure={
                      source.files.length > 1
                        ? { open: isOpen(source.id), onToggle: () => toggle(source.id) }
                        : undefined
                    }
                  />
                  {isOpen(source.id) && source.files.length > 1 && (
                    <ul className="ml-3 flex flex-col gap-0.5 border-l pl-1.5">
                      {source.files.map((file) => (
                        <FileRow
                          key={file.id}
                          file={file}
                          active={scope.view === "file" && scope.id === file.id}
                          onClick={() =>
                            onScope({ view: "file", id: file.id, sourceId: source.id })
                          }
                        />
                      ))}
                    </ul>
                  )}
                </div>
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
                      "inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs transition-colors",
                      activeTags.includes(tag.name)
                        ? "border-transparent bg-primary text-primary-foreground"
                        : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                    )}
                  >
                    <TagIcon className="size-3" />
                    {tag.name}
                    <span className="tabular-nums opacity-60">{tag.count}</span>
                  </button>
                ))}
              </div>
            </Section>
          )}
        </nav>
      </div>

      <Separator />
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
  label,
  count,
  active,
  onClick,
  dim,
  disclosure,
}: {
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  count: number;
  active: boolean;
  onClick: () => void;
  dim?: boolean;
  disclosure?: { open: boolean; onToggle: () => void };
}) {
  return (
    <div className="flex items-center">
      {disclosure ? (
        <button
          type="button"
          onClick={disclosure.onToggle}
          aria-label={disclosure.open ? "Collapse" : "Expand"}
          className="rounded p-0.5 text-muted-foreground hover:text-foreground"
        >
          <ChevronRight
            className={cn("size-3.5 transition-transform", disclosure.open && "rotate-90")}
          />
        </button>
      ) : (
        <span className="w-[1.125rem]" />
      )}
      <button
        type="button"
        onClick={onClick}
        title={label}
        className={cn(
          "flex min-w-0 flex-1 items-center gap-2 rounded-md px-2 py-1.5 text-sm transition-colors",
          active
            ? "bg-sidebar-accent text-sidebar-accent-foreground"
            : "text-muted-foreground hover:bg-sidebar-accent/60 hover:text-sidebar-accent-foreground",
          dim && "opacity-50",
        )}
      >
        <Icon className="size-4 shrink-0" />
        <span className="truncate">{label}</span>
        <span className="ml-auto shrink-0 text-xs tabular-nums opacity-70">{count}</span>
      </button>
    </div>
  );
}

function FileRow({
  file,
  active,
  onClick,
}: {
  file: LibraryFile;
  active: boolean;
  onClick: () => void;
}) {
  const Icon = ROLE_ICON[file.role] ?? FileMusic;

  return (
    <li>
      <button
        type="button"
        onClick={onClick}
        title={`${file.file_name} — ${ROLE_LABEL[file.role]}`}
        className={cn(
          "flex w-full items-center gap-1.5 rounded-md px-2 py-1 text-xs transition-colors",
          active
            ? "bg-sidebar-accent text-sidebar-accent-foreground"
            : "text-muted-foreground hover:bg-sidebar-accent/60 hover:text-sidebar-accent-foreground",
        )}
      >
        <Icon className="size-3.5 shrink-0" />
        <span className="truncate">{fileLabel(file.file_name)}</span>
        {file.status === "invalid" ? (
          <TriangleAlert className="ml-auto size-3 shrink-0 text-destructive" />
        ) : (
          <span className="ml-auto shrink-0 rounded border px-1 text-[9px] tracking-wide uppercase opacity-70">
            {ROLE_LABEL[file.role]}
          </span>
        )}
      </button>
    </li>
  );
}

const ROLE_LABEL: Record<Role, string> = {
  backup: "backup",
  "scene-bank": "scenes",
  "tone-bank": "tones",
  "sample-bank": "samples",
  unknown: "?",
};

