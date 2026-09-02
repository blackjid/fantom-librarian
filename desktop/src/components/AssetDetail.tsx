import { useEffect, useState } from "react";
import { AlertTriangle, Archive, Check, Package, Pencil, Plus, Tag as TagIcon, X } from "lucide-react";
import { api, message, type Asset, type SceneDetail, type ZoneState } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Separator } from "@/components/ui/separator";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { date, note, pan, plural, range, signed } from "@/lib/format";

/**
 * The right-hand pane. Everything the library knows about one asset: what it is, what it needs,
 * where it came from, and the handful of things v1 lets you change about it.
 */
export function AssetDetail({
  asset,
  onChanged,
}: {
  asset: Asset;
  onChanged: () => void;
}) {
  const [error, setError] = useState<string | null>(null);

  return (
    <div className="flex h-full min-w-[22rem] flex-1 flex-col">
      <Header asset={asset} onChanged={onChanged} onError={setError} />
      {error && (
        <Alert variant="destructive" className="mx-4 mb-2 w-auto">
          <AlertTitle>That change was not saved</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <Tabs defaultValue="overview" className="flex min-h-0 flex-1 flex-col">
        <TabsList className="mx-4">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          {asset.detail.kind === "scene" && <TabsTrigger value="zones">Zones</TabsTrigger>}
          <TabsTrigger value="sources">
            Sources
            <Badge variant="secondary" className="ml-1.5 text-[10px]">
              {asset.sources.length}
            </Badge>
          </TabsTrigger>
        </TabsList>

        <div className="scroll-region min-h-0 flex-1">
          <TabsContent value="overview" className="m-0 flex flex-col gap-5 p-4">
            <Overview asset={asset} onChanged={onChanged} onError={setError} />
          </TabsContent>

          {asset.detail.kind === "scene" && (
            <TabsContent value="zones" className="m-0 p-4">
              <Zones detail={asset.detail} />
            </TabsContent>
          )}

          <TabsContent value="sources" className="m-0 p-4">
            <Sources asset={asset} />
          </TabsContent>
        </div>
      </Tabs>
    </div>
  );
}

function Header({
  asset,
  onChanged,
  onError,
}: {
  asset: Asset;
  onChanged: () => void;
  onError: (error: string | null) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(asset.fantom_name);
  const [nameError, setNameError] = useState<string | null>(null);

  // A different asset in the same pane must not inherit the last one's half-typed name.
  useEffect(() => {
    setEditing(false);
    setDraft(asset.fantom_name);
    setNameError(null);
  }, [asset.id, asset.fantom_name]);

  // Scene names are hardware-verified, so they are editable. Tone renaming waits for a write
  // path that has been proved on the device.
  const renameable = asset.kind === "scene";

  async function check(value: string) {
    setDraft(value);
    setNameError(await api.checkName(value));
  }

  async function save() {
    onError(null);
    try {
      await api.renameAsset(asset.id, draft);
      setEditing(false);
      onChanged();
    } catch (e) {
      onError(message(e));
    }
  }

  return (
    <div className="flex flex-col gap-2 border-b p-4">
      <div className="flex items-start gap-3">
        <div className="flex min-w-0 flex-1 flex-col gap-1">
          {editing ? (
            <div className="flex items-center gap-2">
              <Input
                value={draft}
                onChange={(e) => void check(e.target.value)}
                aria-invalid={Boolean(nameError)}
                autoFocus
                className="max-w-xs font-mono"
              />
              <Button size="sm" onClick={save} disabled={Boolean(nameError)}>
                <Check data-icon="inline-start" />
                Save
              </Button>
              <Button size="sm" variant="ghost" onClick={() => setEditing(false)}>
                Cancel
              </Button>
            </div>
          ) : (
            <div className="flex items-center gap-2">
              <h1 className="truncate text-lg font-semibold" data-selectable>
                {asset.fantom_name}
              </h1>
              {renameable && (
                <Button
                  size="icon-sm"
                  variant="ghost"
                  onClick={() => setEditing(true)}
                  aria-label="Rename"
                >
                  <Pencil />
                </Button>
              )}
            </div>
          )}
          {nameError && <p className="text-xs text-destructive">{nameError}</p>}
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <Badge
              variant="outline"
              className={asset.kind === "scene" ? "text-scene" : "text-tone"}
            >
              {asset.kind}
            </Badge>
            <span>{asset.engine}</span>
            {asset.detail.kind === "scene" && (
              <>
                <span>·</span>
                <span>{asset.detail.bpm.toFixed(2)} BPM</span>
                <span>·</span>
                <span>level {asset.detail.level}</span>
              </>
            )}
            {asset.imported_name !== asset.fantom_name && (
              <>
                <span>·</span>
                <span>imported as “{asset.imported_name}”</span>
              </>
            )}
          </div>
        </div>

        <Button
          variant="outline"
          size="sm"
          onClick={async () => {
            onError(null);
            try {
              await api.archiveAsset(asset.id, !asset.archived_at);
              onChanged();
            } catch (e) {
              onError(message(e));
            }
          }}
        >
          <Archive data-icon="inline-start" />
          {asset.archived_at ? "Restore" : "Archive"}
        </Button>
      </div>
    </div>
  );
}

function Overview({
  asset,
  onChanged,
  onError,
}: {
  asset: Asset;
  onChanged: () => void;
  onError: (error: string | null) => void;
}) {
  return (
    <>
      <Tags asset={asset} onChanged={onChanged} onError={onError} />

      {asset.memo && (
        <Block title="FANTOM memo" hint="Preserved as imported; not editable in this version.">
          <p className="rounded-md bg-muted/50 p-3 font-mono text-sm" data-selectable>
            {asset.memo}
          </p>
        </Block>
      )}

      <Note asset={asset} onChanged={onChanged} onError={onError} />

      {asset.detail.kind === "scene" && (
        <>
          <Block
            title="User tones it needs"
            hint="Included automatically when this scene goes into a package. Counted from every zone that can sound — muted, switched off, or recalled by a keyboard group — never from one left at the factory default."
          >
            {asset.detail.user_tones.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                None — every zone that can sound plays factory or expansion content.
              </p>
            ) : (
              <ul className="flex flex-col gap-1">
                {asset.detail.user_tones.map((tone) => (
                  <li key={tone} className="flex items-center gap-2 text-sm">
                    <Package className="size-3.5 text-muted-foreground" />
                    <span className="font-mono">{tone}</span>
                  </li>
                ))}
              </ul>
            )}
          </Block>

          {asset.detail.external_refs.length > 0 && (
            <Block
              title="External requirements"
              hint="Never substituted. These have to be present on the instrument."
            >
              <Alert>
                <AlertTriangle />
                <AlertTitle>
                  {plural(asset.detail.external_refs.length, "reference")} outside your library
                </AlertTitle>
                <AlertDescription>
                  <ul className="flex flex-col gap-0.5 font-mono text-xs">
                    {asset.detail.external_refs.map((ref) => (
                      <li key={ref}>{ref}</li>
                    ))}
                  </ul>
                </AlertDescription>
              </Alert>
            </Block>
          )}
        </>
      )}
    </>
  );
}

function Tags({
  asset,
  onChanged,
  onError,
}: {
  asset: Asset;
  onChanged: () => void;
  onError: (error: string | null) => void;
}) {
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState("");

  async function add() {
    const value = draft.trim();
    if (!value) return setAdding(false);
    onError(null);
    try {
      await api.addTag(asset.id, value);
      setDraft("");
      setAdding(false);
      onChanged();
    } catch (e) {
      onError(message(e));
    }
  }

  return (
    <Block title="Tags">
      <div className="flex flex-wrap items-center gap-1.5">
        {asset.tags.map((tag) => (
          <Badge key={tag} variant="secondary" className="gap-1">
            <TagIcon className="size-3" />
            {tag}
            <button
              type="button"
              aria-label={`Remove ${tag}`}
              className="relative rounded-full after:absolute after:-inset-[6px] after:content-['']"
              onClick={async () => {
                onError(null);
                try {
                  await api.removeTag(asset.id, tag);
                  onChanged();
                } catch (e) {
                  onError(message(e));
                }
              }}
            >
              <X className="size-3" />
            </button>
          </Badge>
        ))}

        {adding ? (
          <Input
            value={draft}
            autoFocus
            onChange={(e) => setDraft(e.target.value)}
            onBlur={add}
            onKeyDown={(e) => {
              if (e.key === "Enter") void add();
              if (e.key === "Escape") setAdding(false);
            }}
            className="h-6 w-32 text-xs"
            placeholder="new tag"
          />
        ) : (
          <Button size="xs" variant="outline" onClick={() => setAdding(true)}>
            <Plus data-icon="inline-start" />
            Add
          </Button>
        )}
      </div>
    </Block>
  );
}

function Note({
  asset,
  onChanged,
  onError,
}: {
  asset: Asset;
  onChanged: () => void;
  onError: (error: string | null) => void;
}) {
  const [draft, setDraft] = useState(asset.note);
  useEffect(() => setDraft(asset.note), [asset.id, asset.note]);
  const dirty = draft !== asset.note;

  async function save(value: string) {
    onError(null);
    try {
      await api.setAssetNote(asset.id, value);
      onChanged();
    } catch (e) {
      onError(message(e));
    }
  }

  return (
    <Block title="Library note" hint="Yours, as long as you like. Never written to a FANTOM file.">
      <Textarea
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        // Leaving the field commits it: selecting another asset resets the draft.
        onBlur={() => {
          if (draft !== asset.note) void save(draft);
        }}
        rows={3}
        placeholder="What this is for, where it works, what you changed…"
      />
      {/* Both hold focus in the field, so pressing one does not fire the blur save first. */}
      {dirty && (
        <div className="flex gap-2">
          <Button size="sm" onMouseDown={(e) => e.preventDefault()} onClick={() => void save(draft)}>
            Save note
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onMouseDown={(e) => e.preventDefault()}
            onClick={() => setDraft(asset.note)}
          >
            Discard
          </Button>
        </div>
      )}
    </Block>
  );
}

/** How each zone state reads, and what it means for a package. */
const ZONE_STATE: Record<ZoneState, { label: string; hint: string; className: string }> = {
  on: { label: "on", hint: "Switched on and sounding.", className: "text-scene" },
  muted: { label: "mute", hint: "Switched on but muted.", className: "text-scene/70" },
  grouped: {
    label: "group",
    hint: "Switched off now, but a keyboard group switches it on.",
    className: "text-tone",
  },
  off: {
    label: "off",
    hint: "Set up and then switched off — silent, but still part of the scene.",
    className: "",
  },
  unused: {
    label: "—",
    hint: "Never configured: still exactly as the factory left it.",
    className: "",
  },
};

function Zones({ detail }: { detail: SceneDetail }) {
  const { zones, groups } = detail;
  return (
    <div className="flex flex-col gap-4">
      {groups.length > 0 && (
        <section className="flex flex-col gap-2">
          <div className="flex flex-col gap-0.5">
            <h3 className="text-xs font-medium tracking-wider text-muted-foreground uppercase">
              Keyboard groups
            </h3>
            <p className="text-xs text-muted-foreground">
              Each pad recalls one set of zone switches, so this scene holds several arrangements.
              A zone below marked <span className="text-tone">group</span> is off right now and
              plays when its group is selected.
            </p>
          </div>
          <div className="flex flex-wrap gap-1.5">
            {groups.map((group) => (
              <span
                key={group.number}
                className="inline-flex items-center gap-1.5 rounded-md border px-2 py-1 font-mono text-xs"
              >
                <span className="text-muted-foreground">G{group.number}</span>
                <span>{group.zones.join(" ")}</span>
              </span>
            ))}
          </div>
        </section>
      )}

      {/* Fourteen columns outrun a narrow pane; the permanent gutter says the rest are there. */}
      <div className="scroll-region-x overflow-x-auto rounded-md border" data-selectable>
      <table className="w-full text-xs">
        <thead className="bg-muted/50 text-muted-foreground">
          <tr>
            {["#", "state", "grp", "engine", "bank", "tone", "key", "vel", "lvl", "pan", "tr", "oct", "ch", "arp"].map(
              (head) => (
                <th key={head} className="px-2 py-1.5 text-left font-medium whitespace-nowrap">
                  {head}
                </th>
              ),
            )}
          </tr>
        </thead>
        <tbody className="font-mono">
          {zones.map((zone) => (
            <tr
              key={zone.number}
              className={cn(
                // An unused zone still holds real values, so it stays legible: `muted-foreground`
                // against `foreground` is hierarchy enough without a second opacity layer.
                "border-t",
                zone.state === "unused" ? "text-muted-foreground" : "text-foreground",
              )}
            >
              <td className="px-2 py-1 tabular-nums">{zone.number}</td>
              <td
                className={cn("px-2 py-1", ZONE_STATE[zone.state].className)}
                title={ZONE_STATE[zone.state].hint}
              >
                {ZONE_STATE[zone.state].label}
              </td>
              <td className="px-2 py-1 whitespace-nowrap text-tone">
                {zone.groups.length > 0 ? zone.groups.join(",") : ""}
              </td>
              <td className="px-2 py-1 whitespace-nowrap">{zone.engine}</td>
              <td className={cn("px-2 py-1 whitespace-nowrap", zone.bank === "USER" && "text-tone")}>
                {zone.bank}
              </td>
              <td className="max-w-40 truncate px-2 py-1" title={zone.tone}>
                {zone.tone}
              </td>
              <td className="px-2 py-1 whitespace-nowrap">
                {note(zone.key_low)}–{note(zone.key_high)}
              </td>
              <td className="px-2 py-1 whitespace-nowrap">
                {range(zone.velocity_low, zone.velocity_high)}
              </td>
              <td className="px-2 py-1 tabular-nums">{zone.level}</td>
              <td className="px-2 py-1">{pan(zone.pan)}</td>
              <td className="px-2 py-1 tabular-nums">{signed(zone.transpose)}</td>
              <td className="px-2 py-1 tabular-nums">{signed(zone.octave)}</td>
              <td className="px-2 py-1 tabular-nums">{zone.midi_channel}</td>
              <td className="px-2 py-1">{zone.arpeggio ? "on" : "—"}</td>
            </tr>
          ))}
        </tbody>
        </table>
      </div>
    </div>
  );
}

function Sources({ asset }: { asset: Asset }) {
  return (
    <div className="flex flex-col gap-3">
      <p className="text-xs text-muted-foreground">
        The same material can arrive in several packs. Each one keeps its own record of where this
        was found; the library shows one canonical item.
      </p>
      {asset.sources.map((source) => (
        <div key={`${source.file_id}-${source.slot}`} className="rounded-md border p-3">
          <div className="flex items-center justify-between gap-2">
            <span className="truncate text-sm font-medium">{source.source_name}</span>
            <Badge variant="outline" className="shrink-0 font-mono text-[10px]">
              {asset.kind === "scene" ? `scene ${source.slot}` : `${source.area}[${source.slot}]`}
            </Badge>
          </div>
          <Separator className="my-2" />
          <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs">
            <dt className="text-muted-foreground">File</dt>
            <dd className="truncate font-mono">{source.file_name}</dd>
            <dt className="text-muted-foreground">Named</dt>
            <dd className="truncate font-mono">{source.name_at_import}</dd>
          </dl>
        </div>
      ))}
      <p className="text-xs text-muted-foreground">Added to the library {date(asset.created_at)}.</p>
    </div>
  );
}

function Block({
  title,
  hint,
  children,
}: {
  title: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <section className="flex flex-col gap-2">
      <div className="flex flex-col gap-0.5">
        <h3 className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
          {title}
        </h3>
        {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
      </div>
      {children}
    </section>
  );
}
