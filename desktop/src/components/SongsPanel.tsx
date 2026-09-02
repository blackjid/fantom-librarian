import { useState } from "react";
import { Music4, Plus, Trash2, Unlink } from "lucide-react";
import { api, message, type Asset, type Song } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Field, FieldDescription, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty";
import { plural } from "@/lib/format";

/**
 * Songs are cover-band context, not a setlist manager: a song points at the scenes and tones that
 * suit it, and nothing about a song is ever written into a FANTOM file.
 */
export function SongsPanel({
  songs,
  selectedId,
  onSelect,
  onChanged,
  linkTarget,
}: {
  songs: Song[];
  selectedId: number | null;
  onSelect: (song: Song | null) => void;
  onChanged: () => void;
  /** The asset selected in the library, offered as a one-click link. */
  linkTarget: Asset | null;
}) {
  const [creating, setCreating] = useState(false);
  const selected = songs.find((song) => song.id === selectedId) ?? null;

  return (
    <div className="surface flex h-full min-w-0 flex-1 overflow-hidden rounded-2xl bg-panel">
      <div className="flex w-72 min-w-[15rem] flex-col">
        <div className="flex items-center justify-between gap-2 border-b p-3">
          <h2 className="text-sm font-medium">Songs</h2>
          <Button size="sm" variant="outline" onClick={() => setCreating(true)}>
            <Plus data-icon="inline-start" />
            New
          </Button>
        </div>
        <div className="scroll-region flex-1">
          {songs.length === 0 ? (
            <p className="p-4 text-xs text-muted-foreground">
              No songs yet. A song links the sounds you use for it, so a Rhodes tone can belong to
              a song before a scene does.
            </p>
          ) : (
            <ul className="p-1.5">
              {songs.map((song) => (
                <li key={song.id}>
                  <button
                    type="button"
                    onClick={() => {
                      setCreating(false);
                      onSelect(song);
                    }}
                    className={cn(
                      "flex w-full flex-col gap-0.5 rounded-md px-2 py-2 text-left transition-colors",
                      song.id === selectedId ? "bg-accent" : "hover:bg-accent/50",
                    )}
                  >
                    <span className="truncate text-sm font-medium">{song.title}</span>
                    <span className="truncate text-xs text-muted-foreground">
                      {[song.artist, song.song_key].filter(Boolean).join(" · ") || "—"}
                    </span>
                    {song.links.length > 0 && (
                      <span className="text-[10px] text-muted-foreground">
                        {plural(song.links.length, "sound")}
                      </span>
                    )}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
      </div>

      <div className="surface m-2 min-w-[20rem] flex-1 overflow-hidden rounded-lg bg-panel-raised">
        {creating ? (
          <SongForm
            onDone={(created) => {
              setCreating(false);
              onChanged();
              if (created) onSelect(null);
            }}
          />
        ) : selected ? (
          <SongDetail
            song={selected}
            onChanged={onChanged}
            linkTarget={linkTarget}
            onDeleted={() => {
              onSelect(null);
              onChanged();
            }}
          />
        ) : (
          <Empty className="h-full">
            <EmptyHeader>
              <EmptyMedia variant="icon">
                <Music4 />
              </EmptyMedia>
              <EmptyTitle>No song selected</EmptyTitle>
              <EmptyDescription>
                Pick a song, or create one and link the scenes and tones you play it with.
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        )}
      </div>
    </div>
  );
}

function SongForm({ onDone }: { onDone: (created: boolean) => void }) {
  const [title, setTitle] = useState("");
  const [artist, setArtist] = useState("");
  const [key, setKey] = useState("");
  const [notes, setNotes] = useState("");
  const [error, setError] = useState<string | null>(null);

  async function save() {
    setError(null);
    try {
      await api.createSong({ title, artist, song_key: key, notes });
      onDone(true);
    } catch (e) {
      setError(message(e));
    }
  }

  return (
    <div className="flex flex-col gap-4 p-6">
      <h2 className="text-lg font-semibold">New song</h2>
      <FieldGroup>
        <Field>
          <FieldLabel htmlFor="song-title">Title</FieldLabel>
          <Input id="song-title" value={title} onChange={(e) => setTitle(e.target.value)} autoFocus />
        </Field>
        <div className="grid gap-4 sm:grid-cols-2">
          <Field>
            <FieldLabel htmlFor="song-artist">Original artist</FieldLabel>
            <Input id="song-artist" value={artist} onChange={(e) => setArtist(e.target.value)} />
          </Field>
          <Field>
            <FieldLabel htmlFor="song-key">Performance key</FieldLabel>
            <Input id="song-key" value={key} onChange={(e) => setKey(e.target.value)} />
            <FieldDescription>The key you play it in, which is often not the original.</FieldDescription>
          </Field>
        </div>
        <Field>
          <FieldLabel htmlFor="song-notes">Notes</FieldLabel>
          <Textarea id="song-notes" value={notes} onChange={(e) => setNotes(e.target.value)} rows={3} />
        </Field>
      </FieldGroup>

      {error && (
        <Alert variant="destructive">
          <AlertTitle>Could not save</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <div className="flex gap-2">
        <Button onClick={save}>Create song</Button>
        <Button variant="ghost" onClick={() => onDone(false)}>
          Cancel
        </Button>
      </div>
    </div>
  );
}

function SongDetail({
  song,
  onChanged,
  onDeleted,
  linkTarget,
}: {
  song: Song;
  onChanged: () => void;
  onDeleted: () => void;
  linkTarget: Asset | null;
}) {
  const alreadyLinked = linkTarget
    ? song.links.some((link) => link.asset_id === linkTarget.id)
    : false;
  const [confirming, setConfirming] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /** Every mutation here reports: a swallowed rejection looks exactly like a change that worked. */
  async function run(action: () => Promise<unknown>, after: () => void) {
    setError(null);
    try {
      await action();
      after();
    } catch (e) {
      setError(message(e));
    }
  }

  return (
    <div className="scroll-region h-full">
      <div className="flex flex-col gap-5 p-6">
        <div className="flex items-start justify-between gap-3">
          <div className="flex min-w-0 flex-col gap-1">
            <h2 className="truncate text-lg font-semibold">{song.title}</h2>
            <p className="text-sm text-muted-foreground">
              {[song.artist, song.song_key && `key of ${song.song_key}`]
                .filter(Boolean)
                .join(" · ") || "No artist or key recorded"}
            </p>
          </div>
          <Button variant="destructive" size="sm" onClick={() => setConfirming(true)}>
            <Trash2 data-icon="inline-start" />
            Delete
          </Button>
        </div>

        {/* Deleting a song takes its notes and every link with it, and nothing here undoes that. */}
        <Dialog open={confirming} onOpenChange={setConfirming}>
          <DialogContent className="sm:max-w-sm">
            <DialogHeader>
              <DialogTitle>Delete “{song.title}”?</DialogTitle>
              <DialogDescription>
                Its notes and {plural(song.links.length, "linked sound")} go with it. The sounds
                themselves stay in your library. This cannot be undone.
              </DialogDescription>
            </DialogHeader>
            <DialogFooter>
              <Button variant="ghost" onClick={() => setConfirming(false)}>
                Cancel
              </Button>
              <Button
                variant="destructive"
                onClick={() =>
                  void run(
                    () => api.deleteSong(song.id),
                    () => {
                      setConfirming(false);
                      onDeleted();
                    },
                  )
                }
              >
                Delete song
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>

        {error && (
          <Alert variant="destructive">
            <AlertTitle>That change was not saved</AlertTitle>
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        {song.notes && <p className="rounded-md bg-muted/50 p-3 text-sm">{song.notes}</p>}

        <section className="flex flex-col gap-2">
          <h3 className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
            Linked sounds
          </h3>
          {song.links.length === 0 ? (
            <p className="text-sm text-muted-foreground">Nothing linked yet.</p>
          ) : (
            <ul className="flex flex-col gap-1">
              {song.links.map((link) => (
                <li
                  key={link.asset_id}
                  className="flex items-center gap-2 rounded-md border px-3 py-2"
                >
                  <Badge variant="outline" className="shrink-0 text-[10px]">
                    {link.asset_kind}
                  </Badge>
                  <span className="truncate text-sm font-medium">{link.asset_name}</span>
                  {link.note && (
                    <span className="truncate text-xs text-muted-foreground">{link.note}</span>
                  )}
                  <Button
                    size="icon-sm"
                    variant="ghost"
                    className="ml-auto shrink-0"
                    aria-label={`Unlink ${link.asset_name}`}
                    onClick={() =>
                      void run(() => api.unlinkSong(song.id, link.asset_id), onChanged)
                    }
                  >
                    <Unlink />
                  </Button>
                </li>
              ))}
            </ul>
          )}

          {linkTarget && !alreadyLinked && (
            <Button
              variant="outline"
              size="sm"
              className="self-start"
              onClick={() => void run(() => api.linkSong(song.id, linkTarget.id), onChanged)}
            >
              <Plus data-icon="inline-start" />
              Link “{linkTarget.fantom_name}”
            </Button>
          )}
        </section>
      </div>
    </div>
  );
}
