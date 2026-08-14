import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FileDown, FolderOpen, Files, X } from "lucide-react";
import { api, message, type ImportReport } from "@/lib/api";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Field, FieldDescription, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Spinner } from "@/components/ui/spinner";
import { plural } from "@/lib/format";

/**
 * One import is one source group, so a pack and the sample material that came with it keep their
 * relationship. Provenance is all optional — an incomplete note never blocks an import.
 */
export function ImportDialog({
  open: isOpen,
  onOpenChange,
  onImported,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImported: (report: ImportReport) => void;
}) {
  const [paths, setPaths] = useState<string[]>([]);
  const [name, setName] = useState("");
  const [vendor, setVendor] = useState("");
  const [url, setUrl] = useState("");
  const [licence, setLicence] = useState("");
  const [note, setNote] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function reset() {
    setPaths([]);
    setName("");
    setVendor("");
    setUrl("");
    setLicence("");
    setNote("");
    setError(null);
  }

  async function pick(directory: boolean) {
    const picked = await open({
      directory,
      multiple: !directory,
      title: directory ? "Choose a pack folder" : "Choose files",
      filters: directory ? undefined : [{ name: "FANTOM files", extensions: ["svd", "svz"] }],
    });
    if (!picked) return;
    const next = Array.isArray(picked) ? picked : [picked];
    setPaths((current) => [...new Set([...current, ...next])]);
  }

  async function run() {
    setBusy(true);
    setError(null);
    try {
      const report = await api.importFiles(paths, {
        name,
        vendor,
        url,
        licence_note: licence,
        note,
      });
      onImported(report);
      reset();
      onOpenChange(false);
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog
      open={isOpen}
      onOpenChange={(next) => {
        if (!next) reset();
        onOpenChange(next);
      }}
    >
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Import a source</DialogTitle>
          <DialogDescription>
            Everything you choose is imported together as one source group. Your original files are
            copied, never changed.
          </DialogDescription>
        </DialogHeader>

        <FieldGroup>
          <Field>
            <FieldLabel>Files</FieldLabel>
            <div className="flex gap-2">
              <Button type="button" variant="outline" size="sm" onClick={() => pick(true)}>
                <FolderOpen data-icon="inline-start" />
                Add folder
              </Button>
              <Button type="button" variant="outline" size="sm" onClick={() => pick(false)}>
                <Files data-icon="inline-start" />
                Add files
              </Button>
            </div>
            {paths.length > 0 && (
              <div className="scroll-region mt-2 max-h-32 rounded-md border">
                <ul className="p-1">
                  {paths.map((path) => (
                    <li
                      key={path}
                      className="flex items-center gap-2 rounded px-2 py-1 text-xs hover:bg-accent"
                    >
                      <span className="truncate font-mono" title={path}>
                        {path}
                      </span>
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-sm"
                        className="ml-auto shrink-0"
                        onClick={() => setPaths((c) => c.filter((p) => p !== path))}
                      >
                        <X />
                        <span className="sr-only">Remove</span>
                      </Button>
                    </li>
                  ))}
                </ul>
              </div>
            )}
            <FieldDescription>
              A folder and the files inside it count as one pack. `.svd` and `.svz` only — Roland
              Cloud `.sdz` is not supported.
            </FieldDescription>
          </Field>

          <Field>
            <FieldLabel htmlFor="source-name">Source name</FieldLabel>
            <Input
              id="source-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Named after the folder if you leave this blank"
            />
          </Field>

          <div className="grid gap-4 sm:grid-cols-2">
            <Field>
              <FieldLabel htmlFor="source-vendor">Author or vendor</FieldLabel>
              <Input
                id="source-vendor"
                value={vendor}
                onChange={(e) => setVendor(e.target.value)}
              />
            </Field>
            <Field>
              <FieldLabel htmlFor="source-url">URL</FieldLabel>
              <Input id="source-url" value={url} onChange={(e) => setUrl(e.target.value)} />
            </Field>
          </div>

          <Field>
            <FieldLabel htmlFor="source-licence">Licence or ownership note</FieldLabel>
            <Input
              id="source-licence"
              value={licence}
              onChange={(e) => setLicence(e.target.value)}
              placeholder="Purchased, free download, personal use only…"
            />
            <FieldDescription>
              Recorded so the app can warn you later; you remain responsible for the rights.
            </FieldDescription>
          </Field>

          <Field>
            <FieldLabel htmlFor="source-note">Note</FieldLabel>
            <Textarea
              id="source-note"
              value={note}
              onChange={(e) => setNote(e.target.value)}
              rows={2}
            />
          </Field>
        </FieldGroup>

        {error && (
          <Alert variant="destructive">
            <AlertTitle>Import failed</AlertTitle>
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        )}

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={busy}>
            Cancel
          </Button>
          <Button onClick={run} disabled={busy || paths.length === 0}>
            {busy ? <Spinner data-icon="inline-start" /> : <FileDown data-icon="inline-start" />}
            Import {paths.length > 0 && plural(paths.length, "item")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
