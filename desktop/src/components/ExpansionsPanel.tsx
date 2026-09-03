import { useEffect, useState } from "react";
import { CircleAlert, HardDriveDownload, Minus, Plus, ShoppingBag } from "lucide-react";
import {
  api,
  message,
  type ExpansionEntry,
  type ExpansionFamily,
  type Holding,
} from "@/lib/api";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { plural } from "@/lib/format";

/** Headings, in the order the list shows them, keyed by `expansions::Family` in the core crate. */
const FAMILIES: { key: ExpansionFamily; title: string; hint: string }[] = [
  {
    key: "wave",
    title: "Wave expansions",
    hint: "EXZ — waves the ZEN-Core engine plays, and the drum kits built on them.",
  },
  {
    key: "super-natural",
    title: "SuperNATURAL",
    hint: "EXSN — the acoustic and electric piano expansions.",
  },
  { key: "model", title: "MODEL expansions", hint: "The modelled instruments." },
  { key: "v-piano", title: "V-Piano", hint: "Played by the V-Piano engine." },
  { key: "other", title: "Other", hint: "Recorded here, and not in any bundled catalog." },
];

/**
 * The rung an expansion is on has to read at a glance, and the default toggle only shades its
 * background — which on this theme is all but invisible next to an unset one. A long list of rows
 * is exactly where that matters, so the chosen rung takes the accent colour outright.
 */
const SET =
  "data-[state=on]:border-primary/50 data-[state=on]:bg-primary/15 data-[state=on]:text-primary";

/** The ladder, in the order it climbs. */
const RUNGS: { value: Holding; label: string; icon: typeof Minus }[] = [
  { value: "unowned", label: "No", icon: Minus },
  { value: "owned", label: "Owned", icon: ShoppingBag },
  { value: "loaded", label: "Loaded", icon: HardDriveDownload },
];

/**
 * How far each expansion has got: not owned, owned, or loaded into a slot.
 *
 * Three rungs, not two flags. The FANTOM's expansion slots are finite, so a player owns more than
 * fits at once — and "you own EXSN03, load it" and "you do not own EXSN03" are different things to
 * be told. Nothing here is read off the instrument: no file says what is loaded, so this is the
 * user's own statement about their setup, kept with the library folder so it travels with it.
 */
export function ExpansionsPanel({ onError }: { onError: (error: string | null) => void }) {
  const [entries, setEntries] = useState<ExpansionEntry[] | null>(null);
  const [adding, setAdding] = useState("");

  // Never clears the error: a write that failed reloads afterwards to resync, and clearing here
  // would wipe the message the user needs before they can read it.
  const reload = async () => {
    try {
      setEntries(await api.listExpansions());
    } catch (error) {
      onError(message(error));
    }
  };

  useEffect(() => {
    void reload();
    // Loaded once when the panel opens; every edit refreshes it itself.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function set(code: string, holding: Holding) {
    try {
      await api.setExpansion(code, holding);
      onError(null);
    } catch (error) {
      onError(message(error));
    }
    // Either way: the list is redrawn from what the catalog actually holds, so a rejected write
    // leaves the toggle showing the truth rather than what was clicked.
    await reload();
  }

  async function add() {
    const code = adding.trim();
    if (!code) return;
    setAdding("");
    await set(code, "owned");
  }

  // Loaded implies owned, so the counts nest rather than compete.
  const owned = entries?.filter((entry) => entry.state !== "unowned").length ?? 0;
  const installed = entries?.filter((entry) => entry.state === "loaded").length ?? 0;

  return (
    <div className="surface flex h-full min-w-0 flex-1 flex-col overflow-hidden rounded-2xl bg-panel">
      <div className="flex items-center justify-between gap-3 border-b p-3">
        <h2 className="text-sm font-medium">Expansions</h2>
        <span className="text-xs text-muted-foreground">
          {owned} owned · {installed} loaded
        </span>
      </div>

      <div className="scroll-region flex-1">
        <div className="flex flex-col gap-6 p-4">
          <Alert>
            <CircleAlert />
            <AlertTitle>Owning one and loading it are different</AlertTitle>
            <AlertDescription>
              The instrument's expansion slots are finite, so you can own more than it holds — and
              only what is loaded plays. No FANTOM file records any of this: it is your own note,
              kept with the library folder.
            </AlertDescription>
          </Alert>

          {entries?.length === 0 && (
            <p className="text-sm text-muted-foreground">
              This build carries no expansion catalogs, so there is nothing to list. You can still
              record a product code below.
            </p>
          )}

          {FAMILIES.map(({ key, title, hint }) => {
            const group = (entries ?? []).filter((entry) => entry.family === key);
            if (group.length === 0) return null;
            return (
              <section key={key} className="flex flex-col gap-2">
                <div className="flex flex-col gap-0.5">
                  <h3 className="text-xs font-medium tracking-wider text-muted-foreground uppercase">
                    {title}
                  </h3>
                  <p className="text-xs text-muted-foreground">{hint}</p>
                </div>
                <ul className="flex flex-col">
                  {group.map((entry) => (
                    <li
                      key={entry.code}
                      className="flex items-center gap-3 rounded-md py-1.5 pr-1 pl-2 hover:bg-accent/40"
                    >
                      <span className="w-24 shrink-0 font-mono text-sm">{entry.code}</span>
                      <span className="min-w-0 flex-1 truncate text-xs text-muted-foreground">
                        {entry.catalogued
                          ? `${entry.engine} · ${plural(entry.sounds, "sound")}`
                          : "no bundled catalog — recorded by hand"}
                      </span>
                      <ToggleGroup
                        type="single"
                        size="sm"
                        variant="outline"
                        value={entry.state}
                        // Radix clears the value when the pressed item is pressed again; a rung is
                        // where the expansion *is*, so re-pressing it changes nothing.
                        onValueChange={(next) =>
                          next && void set(entry.code, next as Holding)
                        }
                        aria-label={`${entry.code} status`}
                      >
                        {RUNGS.map(({ value, label, icon: Icon }) => (
                          <ToggleGroupItem
                            key={value}
                            value={value}
                            aria-label={`${entry.code} ${value}`}
                            className={cn(SET, entry.state === value || "text-muted-foreground")}
                          >
                            <Icon data-icon="inline-start" />
                            {label}
                          </ToggleGroupItem>
                        ))}
                      </ToggleGroup>
                    </li>
                  ))}
                </ul>
              </section>
            );
          })}

          <section className="flex flex-col gap-2 border-t pt-4">
            <div className="flex flex-col gap-0.5">
              <h3 className="text-xs font-medium tracking-wider text-muted-foreground uppercase">
                Something not listed
              </h3>
              <p className="text-xs text-muted-foreground">
                The list above is what this build carries catalogs for. An expansion it has never
                seen can still be recorded by its product code — it just cannot name its sounds.
              </p>
            </div>
            <form
              className="flex items-center gap-2"
              onSubmit={(event) => {
                event.preventDefault();
                void add();
              }}
            >
              <Input
                value={adding}
                onChange={(event) => setAdding(event.target.value)}
                placeholder="EXZ016"
                className="h-8 w-48 font-mono text-sm"
                aria-label="Product code"
              />
              <Button type="submit" size="sm" variant="outline" disabled={!adding.trim()}>
                <Plus data-icon="inline-start" />
                Add as owned
              </Button>
            </form>
          </section>
        </div>
      </div>
    </div>
  );
}
