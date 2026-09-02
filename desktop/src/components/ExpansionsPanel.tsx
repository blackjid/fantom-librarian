import { useEffect, useState } from "react";
import { CircleAlert, HardDriveDownload, Plus, ShoppingBag } from "lucide-react";
import { api, message, type ExpansionEntry, type ExpansionFamily } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Toggle } from "@/components/ui/toggle";
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
 * A set flag has to read as set at a glance, and the default toggle only shades its background —
 * which on this theme is all but invisible next to an unset one. Two flags per row across a long
 * list is exactly where that matters, so a set toggle takes the accent colour outright.
 */
const SET =
  "data-[state=on]:border-primary/50 data-[state=on]:bg-primary/15 data-[state=on]:text-primary";

/**
 * What you own, and what the instrument is holding right now.
 *
 * Two facts, not one. The FANTOM's expansion slots are finite, so a player owns more than fits at
 * once — and "you own EXSN03, load it" and "you do not own EXSN03" are different things to be
 * told. Nothing here is read off the instrument: no file says what is installed, so this is the
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

  async function set(code: string, owned: boolean, installed: boolean) {
    try {
      await api.setExpansion(code, owned, installed);
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
    await set(code, true, false);
  }

  const owned = entries?.filter((entry) => entry.owned).length ?? 0;
  const installed = entries?.filter((entry) => entry.installed).length ?? 0;

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
            <AlertTitle>Owned and loaded are separate</AlertTitle>
            <AlertDescription>
              The instrument's expansion slots are finite, so you can own more than it holds. No
              FANTOM file records either one — this is your own note, kept with the library folder.
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
                      <Toggle
                        size="sm"
                        variant="outline"
                        aria-label={`${entry.code} owned`}
                        pressed={entry.owned}
                        onPressedChange={(next) => void set(entry.code, next, entry.installed)}
                        className={cn(SET, entry.owned || "text-muted-foreground")}
                      >
                        <ShoppingBag data-icon="inline-start" />
                        Owned
                      </Toggle>
                      <Toggle
                        size="sm"
                        variant="outline"
                        aria-label={`${entry.code} loaded`}
                        pressed={entry.installed}
                        onPressedChange={(next) => void set(entry.code, entry.owned, next)}
                        className={cn(SET, entry.installed || "text-muted-foreground")}
                      >
                        <HardDriveDownload data-icon="inline-start" />
                        Loaded
                      </Toggle>
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
