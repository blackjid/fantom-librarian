/** Presentation helpers. The Rust side stores raw values; every unit and label is decided here. */

const NOTES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"] as const;

/** MIDI note number as the FANTOM's panel writes it: 60 is C4. */
export function note(n: number): string {
  const name = NOTES[((n % 12) + 12) % 12] ?? "?";
  return `${name}${Math.floor(n / 12) - 1}`;
}

export function range(low: number, high: number): string {
  return low === high ? String(low) : `${low}–${high}`;
}

/** Zone pan, which the file stores zero-centred. */
export function pan(value: number): string {
  if (value === 0) return "C";
  return value < 0 ? `L${-value}` : `R${value}`;
}

export function signed(value: number): string {
  return value > 0 ? `+${value}` : String(value);
}

export function date(unixSeconds: number): string {
  return new Date(unixSeconds * 1000).toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}


export function plural(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

/**
 * What to call an imported file in one line.
 *
 * The instrument writes every backup and every scene export to the same fixed `FANTOM.SVD`, so
 * the basename identifies nothing — the folder the user filed it under is the real name.
 */
export function fileLabel(path: string): string {
  const parts = path.split("/").filter(Boolean);
  const leaf = parts[parts.length - 1] ?? path;
  if (leaf.toLowerCase() === "fantom.svd" && parts.length > 1) {
    return parts[parts.length - 2] ?? leaf;
  }
  return leaf;
}
