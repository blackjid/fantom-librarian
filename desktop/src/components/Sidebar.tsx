import { useState } from "react";
import {
  Archive,
  ChevronRight,
  Database,
  Disc3,
  FileMusic,
  Import,
  Music4,
  Package,
  Piano,
  Tag as TagIcon,
  TriangleAlert,
  Waves,
} from "lucide-react";
import type { AssetKind, KindCounts, Role, Source, Tag } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { fileLabel } from "@/lib/format";

/** Roles get an icon apiece, because "a backup" and "a scene export" behave nothing alike. */
const ROLE_ICON: Record<Role, React.ComponentType<{ className?: string }>> = {
  backup: Database,
  "scene-bank": FileMusic,
  "tone-bank": Package,
  "sample-bank": Waves,
  unknown: FileMusic,
};

/**
 * Where the list is looking. A scene and a tone are different things to go looking for, so the
 * kind is a destination rather than a filter over a mixed list: the sidebar picks one, and
 * files and tags narrow within it. The imported files are one flat list — a source is the pack
 * they arrived in, not a folder to open.
 */
export type Scope =
  | { view: "library" }
  | { view: "source"; id: number }
  | { view: "file"; id: number; sourceId: number }
  | { view: "songs" };

export function Sidebar({
  width,
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
  /** Dragged by the shell, which owns the bounds. */
  width: number;
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
  // Every imported file, in one run, each still knowing the pack it came from.
  const files = sources.flatMap((source) => source.files.map((file) => ({ file, source })));

  return (
    <div
      style={{ width }}
      className="flex h-full shrink-0 flex-col text-sidebar-foreground"
    >
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

          <Section title={`Sources (${files.length})`} collapsible>
            {files.length === 0 ? (
              <p className="px-2 py-1 text-xs text-muted-foreground">Nothing imported yet.</p>
            ) : (
              files.map(({ file, source }) => (
                <Row
                  key={file.id}
                  icon={fileIcon(file.status, file.role, Boolean(source.archived_at))}
                  iconClassName={file.status === "invalid" ? "text-destructive" : undefined}
                  label={fileLabel(file.file_name)}
                  // Five packs can each hold a `FANTOM.SVD`; the pack names them apart.
                  title={`${source.name} — ${file.file_name}`}
                  count={file.asset_count}
                  active={scope.view === "file" && scope.id === file.id}
                  dim={Boolean(source.archived_at)}
                  onClick={() => onScope({ view: "file", id: file.id, sourceId: source.id })}
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

function Section({
  title,
  collapsible,
  children,
}: {
  title: string;
  /** A long run of files is worth folding away; a wrap of tag chips is not. */
  collapsible?: boolean;
  children: React.ReactNode;
}) {
  const [open, setOpen] = useState(true);
  const heading = "px-2 text-[11px] font-medium tracking-wider text-muted-foreground uppercase";

  if (!collapsible) {
    return (
      <div className="flex flex-col gap-1">
        <h2 className={heading}>{title}</h2>
        {children}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1">
      <h2>
        <button
          type="button"
          onClick={() => setOpen((current) => !current)}
          aria-expanded={open}
          className={cn(heading, "flex w-full items-center gap-1 py-0.5 hover:text-foreground")}
        >
          {title}
          <ChevronRight className={cn("size-3 transition-transform", open && "rotate-90")} />
        </button>
      </h2>
      {open && children}
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
  title,
}: {
  icon: React.ComponentType<{ className?: string }>;
  /** Scenes and tones carry their hue here too, so the nav matches the rows it leads to. */
  iconClassName?: string;
  label: string;
  /** What hovering says, when the label is a shortening of something longer. */
  title?: string;
  count: number;
  active: boolean;
  onClick: () => void;
  dim?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title ?? label}
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

function fileIcon(status: string, role: Role, archived: boolean) {
  if (status === "invalid") return TriangleAlert;
  if (archived) return Archive;
  return ROLE_ICON[role] ?? FileMusic;
}
