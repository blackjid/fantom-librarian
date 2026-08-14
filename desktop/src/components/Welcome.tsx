import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, FolderPlus, Library } from "lucide-react";
import { api, message, type WorkspaceInfo } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Spinner } from "@/components/ui/spinner";

/**
 * Before anything else the app needs one folder to call the library. It is ordinary user data —
 * copyable, backup-able — so choosing it is a plain folder pick, not a hidden app-data location.
 */
export function Welcome({ onOpen }: { onOpen: (info: WorkspaceInfo) => void }) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function choose(create: boolean) {
    setError(null);
    const picked = await open({
      directory: true,
      multiple: false,
      title: create ? "Choose a folder for your library" : "Open a library",
    });
    if (typeof picked !== "string") return;

    setBusy(true);
    try {
      // Opening a folder that is already a library should just work, whichever button was used.
      const existing = await api.isWorkspace(picked);
      onOpen(await api.openWorkspace(picked, create || !existing));
    } catch (e) {
      setError(message(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="flex h-screen flex-col items-center justify-center gap-8 p-10">
      <div className="flex flex-col items-center gap-3 text-center">
        <Library className="size-10 text-muted-foreground" />
        <h1 className="text-2xl font-semibold tracking-tight">FANTOM Librarian</h1>
        <p className="max-w-md text-sm text-muted-foreground">
          One folder holds your whole library: the packs and backups you import, the catalog that
          makes them searchable, and the packages you export. Copy it, back it up, move it between
          machines — it is just a folder.
        </p>
      </div>

      <div className="flex flex-col gap-2 sm:flex-row">
        <Button onClick={() => choose(true)} disabled={busy}>
          {busy ? <Spinner data-icon="inline-start" /> : <FolderPlus data-icon="inline-start" />}
          New library
        </Button>
        <Button variant="outline" onClick={() => choose(false)} disabled={busy}>
          <FolderOpen data-icon="inline-start" />
          Open existing
        </Button>
      </div>

      {error && (
        <Alert variant="destructive" className="max-w-md">
          <AlertTitle>Could not open that folder</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}
    </div>
  );
}
