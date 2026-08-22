/**
 * The bridge to the Rust side.
 *
 * Every type here mirrors a `serde` type in `fantom-library`, and every function wraps one
 * `#[tauri::command]`. Nothing else in the front end calls `invoke` directly, so if a command's
 * shape changes there is exactly one file to fix.
 */
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type AssetKind = "scene" | "tone";
export type FileStatus = "ok" | "invalid";

/**
 * What a file is for. A whole-instrument backup and a three-scene export are both `.svd`, so the
 * extension alone cannot tell them apart — see `role.rs`.
 */
export type Role = "backup" | "scene-bank" | "tone-bank" | "sample-bank" | "unknown";

/** Why a zone sounds, or does not. */
export type ZoneState = "on" | "muted" | "grouped" | "off" | "unused";

/** A saved set of zone switches the player recalls from a pad. */
export interface KeyboardGroup {
  number: number;
  /** 1-based zone numbers this group switches on. */
  zones: number[];
}

export interface ZoneDetail {
  number: number;
  enabled: boolean;
  muted: boolean;
  state: ZoneState;
  /** Keyboard groups that switch this zone on, if any. */
  groups: number[];
  engine: string;
  bank: string;
  tone: string;
  msb: number;
  lsb: number;
  pc: number;
  key_low: number;
  key_high: number;
  velocity_low: number;
  velocity_high: number;
  level: number;
  pan: number;
  transpose: number;
  octave: number;
  midi_channel: number;
  arpeggio: boolean;
}

/** One user sample or multisample slot an asset plays. Slots are 1-based panel numbers. */
export interface SlotRequirement {
  slot: number;
  /** Null when the file that needs the slot does not carry the directory naming it. */
  name: string | null;
  /** Whether the file carries the content for the slot itself. */
  carried: boolean;
  /** Bundled tones that play it. */
  played_by: string[];
}

export interface ToneRequirement {
  area: string;
  index: number;
  engine: string;
  name: string | null;
  address: { msb: number; lsb: number; pc: number };
  /** False when the file points at the slot but bundles no sound there. */
  present: boolean;
}

export interface BankRequirement {
  engine: string;
  /** Bank label when confirmed — `EXZ007`, `JP8`, `PR-A`. Null leaves the raw address to speak. */
  bank: string | null;
  tone: string | null;
  address: { msb: number; lsb: number; pc: number };
}

/** What an asset needs from wherever it is loaded. Mirrors `fantom_core::requirements`. */
export interface Requirements {
  engines: string[];
  user_tones: ToneRequirement[];
  banks: BankRequirement[];
  samples: SlotRequirement[];
  multisamples: SlotRequirement[];
  /** Wave-group ids of installed expansions; the panel's own numbering is not decoded. */
  wave_expansions: number[];
  unclassified: { msb: number; lsb: number; pc: number }[];
  carries_audio: boolean;
}

export interface SceneDetail {
  kind: "scene";
  bpm: number;
  level: number;
  active_zones: number;
  zones: ZoneDetail[];
  engines: string[];
  /** Empty when the scene leaves its keyboard groups at the factory default. */
  groups: KeyboardGroup[];
  user_tones: string[];
  external_refs: string[];
  requirements: Requirements;
}

export interface ToneDetail {
  kind: "tone";
  engine: string;
  area: string;
  index: number;
  requirements: Requirements;
}

export type AssetDetail = SceneDetail | ToneDetail;

export interface AssetSource {
  source_id: number;
  source_name: string;
  file_id: number;
  file_name: string;
  slot: number;
  area: string;
  name_at_import: string;
}

export interface Asset {
  id: number;
  kind: AssetKind;
  fantom_name: string;
  imported_name: string;
  note: string;
  memo: string;
  engine: string;
  detail: AssetDetail;
  created_at: number;
  archived_at: number | null;
  tags: string[];
  sources: AssetSource[];
}

export interface Source {
  id: number;
  name: string;
  vendor: string;
  url: string;
  licence_note: string;
  note: string;
  origin_path: string;
  imported_at: number;
  archived_at: number | null;
  file_count: number;
  asset_count: number;
  files: LibraryFile[];
}

export interface LibraryFile {
  id: number;
  source_id: number;
  /** Path within the import, so five nested `FANTOM.SVD`s stay distinguishable. */
  file_name: string;
  origin_path: string;
  content_hash: string;
  size: number;
  stored_path: string;
  /** The file extension, lowercased. */
  kind: string;
  role: Role;
  status: FileStatus;
  problems: string[];
  asset_count: number;
  sample_count: number;
}

export interface SourceInfo {
  name: string;
  vendor: string;
  url: string;
  licence_note: string;
  note: string;
}

export interface ImportReport {
  source_id: number;
  source_name: string;
  files_imported: number;
  files_skipped: number;
  files_invalid: number;
  scenes_added: number;
  tones_added: number;
  assets_consolidated: number;
  samples_catalogued: number;
  warnings: string[];
}

export interface Tag {
  name: string;
  count: number;
}

export interface SongLink {
  asset_id: number;
  asset_name: string;
  asset_kind: AssetKind;
  note: string;
}

export interface Song {
  id: number;
  title: string;
  artist: string;
  song_key: string;
  notes: string;
  created_at: number;
  links: SongLink[];
}

export interface Query {
  search?: string;
  kind?: AssetKind | null;
  source_id?: number | null;
  /** Narrower than `source_id`: one file within a source. */
  file_id?: number | null;
  song_id?: number | null;
  tags?: string[];
  include_archived?: boolean;
  limit?: number | null;
}

/** Scene and tone totals for a scope, before its kind filter narrows the list. */
export interface KindCounts {
  scenes: number;
  tones: number;
}

export interface Stats {
  scenes: number;
  tones: number;
  sources: number;
  songs: number;
  samples: number;
}

export interface WorkspaceInfo {
  path: string;
  name: string;
  stats: Stats;
}

export const api = {
  openWorkspace: (path: string, create: boolean) =>
    invoke<WorkspaceInfo>("open_workspace", { path, create }),
  resumeWorkspace: () => invoke<WorkspaceInfo | null>("resume_workspace"),
  workspaceInfo: () => invoke<WorkspaceInfo>("workspace_info"),
  closeWorkspace: () => invoke<void>("close_workspace"),
  isWorkspace: (path: string) => invoke<boolean>("is_workspace", { path }),

  importFiles: (paths: string[], info: SourceInfo) =>
    invoke<ImportReport>("import_files", { paths, info }),

  listAssets: (query: Query) => invoke<Asset[]>("list_assets", { query }),
  countAssets: (query: Query) => invoke<KindCounts>("count_assets", { query }),
  getAsset: (id: number) => invoke<Asset>("get_asset", { id }),
  listSources: (includeArchived = false) =>
    invoke<Source[]>("list_sources", { includeArchived }),
  listFiles: (sourceId: number) => invoke<LibraryFile[]>("list_files", { sourceId }),
  listTags: () => invoke<Tag[]>("list_tags"),
  listSongs: (search = "") => invoke<Song[]>("list_songs", { search }),
  getStats: () => invoke<Stats>("get_stats"),

  renameAsset: (id: number, name: string) => invoke<void>("rename_asset", { id, name }),
  setAssetNote: (id: number, note: string) => invoke<void>("set_asset_note", { id, note }),
  archiveAsset: (id: number, archived: boolean) =>
    invoke<void>("archive_asset", { id, archived }),
  archiveSource: (id: number, archived: boolean) =>
    invoke<void>("archive_source", { id, archived }),
  addTag: (assetId: number, tag: string) => invoke<void>("add_tag", { assetId, tag }),
  removeTag: (assetId: number, tag: string) => invoke<void>("remove_tag", { assetId, tag }),
  /** The reason a name would be rejected, or null when the FANTOM would take it. */
  checkName: (name: string) => invoke<string | null>("check_name", { name }),

  createSong: (song: Omit<Song, "id" | "created_at" | "links">) =>
    invoke<number>("create_song", { song }),
  updateSong: (id: number, song: Omit<Song, "id" | "created_at" | "links">) =>
    invoke<void>("update_song", { id, song }),
  deleteSong: (id: number) => invoke<void>("delete_song", { id }),
  linkSong: (songId: number, assetId: number, note = "") =>
    invoke<void>("link_song", { songId, assetId, note }),
  unlinkSong: (songId: number, assetId: number) =>
    invoke<void>("unlink_song", { songId, assetId }),
};

/** Tauri rejects with whatever the command returned; ours are always strings. */
export function message(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}

/** Menu items that stand in for something the window can already do. */
export type MenuAction =
  | "open-library"
  | "close-library"
  | "import"
  | "reveal-library"
  | "find";

/**
 * Run `handler` when a menu item is chosen.
 *
 * The menu emits rather than acting, so a shortcut and a button end up in the same code. Returns
 * the unsubscribe function Tauri hands back.
 */
export function onMenu(handler: (action: MenuAction) => void) {
  return listen<MenuAction>("menu", (event) => handler(event.payload));
}
