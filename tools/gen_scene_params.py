#!/usr/bin/env python3
"""Generate the scene parameter table from Roland's FANTOM EX MIDI Implementation.

    pdftotext -layout fixtures/FANTOM_EX_MIDI_Imple_eng01_W.pdf - \
        | python3 tools/gen_scene_params.py > crates/fantom-core/src/params/scene.rs

The tone table has a different source (`tools/gen_params.py`, from Roland's editor data for
other ZEN-Core instruments). That source models tones, drum kits and the MODEL engines and has
no scene group at all, so the scene map comes from the instrument's own documentation instead.

The PDF gives SysEx addresses. File offsets are *derived* from them by the packing rule in
`docs/FORMAT.md` -- a multi-nibble wire field is one little-endian integer in the file, and the
file pads it to its own alignment -- and every derived offset the repository has independent
evidence for is asserted against that evidence by the tests in `params::scene`.
"""

import re
import sys

# Blocks reachable from a scene, and the wire address each is addressed at. The 16-fold tables
# take a `count` and the address stride between consecutive entries.
SCENE_BLOCKS = [
    # (block name, rust ident, sysex offset, count, addr stride)
    ("Scene Common", "SCENE_COMMON", (0x00, 0x00, 0x00), 1, 0),
    ("Chorus", "CHORUS", (0x00, 0x02, 0x00), 1, 0),
    ("Reverb", "REVERB", (0x00, 0x03, 0x00), 1, 0),
    ("MFX", "IFX", (0x00, 0x04, 0x00), 2, 0x02),
    ("Scene Zone", "SCENE_ZONE", (0x00, 0x10, 0x00), 16, 0x01),
    ("Zone EQ", "ZONE_EQ", (0x00, 0x20, 0x00), 16, 0x01),
    ("Zone Control", "ZONE_CONTROL", (0x00, 0x30, 0x00), 16, 0x01),
    ("Scene Controller", "SCENE_CONTROLLER", (0x00, 0x40, 0x00), 1, 0),
    ("Analog Filter", "ANALOG_FILTER", (0x00, 0x43, 0x00), 1, 0),
]

# Where each block sits in the 3572-byte PRFa record, and how much room it gets.
#
# The lengths are **measured**, not derived. A block's file length is its packed parameter length
# rounded up to some padding the wire map does not predict: `MFX` packs into 80 bytes and occupies
# 84, a figure the independently-sourced tone table arrives at too, and `Zone Control` packs into
# 98 addresses' worth of fields but occupies 96 because its trailing Reserved run is not stored.
# So the generator takes the length as given and asserts only that the parameters fit.
#
# Laid end to end these cover the record exactly, with no gap and no overlap -- which is the
# check that the whole map is right, and is asserted below.
PLACEMENT = {
    "SCENE_COMMON": (0x000, 144),
    "CHORUS": (0x090, 48),
    "REVERB": (0x0C0, 44),
    "IFX": (0x0EC, 84),
    "SCENE_ZONE": (0x194, 72),
    "ZONE_EQ": (0x614, 12),
    "ZONE_CONTROL": (0x6D4, 96),
    "SCENE_CONTROLLER": (0xCD4, 256),
    "ANALOG_FILTER": (0xDD4, 32),
}

RECORD_LEN = 3572

LINE = re.compile(r"^\|\s*(#?)\s*([0-9A-F]{2})\s+([0-9A-F]{2})\s*\|\s*([01a-z][01a-z ]{7,8})\s*\|(.*)$")
TOTAL = re.compile(r"^\|\s*([0-9A-F]{2})\s+([0-9A-F]{2})\s+([0-9A-F]{2})\s+([0-9A-F]{2})\s*\|\s*Total Size")
HEADING = re.compile(r"^\* \[(.+?)\]")

# Roland writes a range as `(lo - hi)` and, for a signed field, follows it with the displayed
# range on the next line. The file stores such a field zero-centred, so the bias is the offset
# from the stored value to the wire value: `Zone Coarse Tune (16 - 112)` shown as `-48 - 48`
# stores -48 as 0 and biases by 64.
#
# Pan uses `L64 - 63R` for the same thing. A display carrying anything else -- an enum, or a
# `0 - 127, TONE` that reserves a value -- is not a plain remap and gets no bias.
RANGE = re.compile(r"\((-?\d+)\s*-\s*(-?\+?\d+)\)")
DISPLAY = re.compile(
    r"^[\s|]*(?:L(\d+)|([+-]?\d+))\s*-\s*(?:(\d+)R|([+-]?\d+))\s*(\[[^\]]*\])?\s*\|?\s*$"
)


def display_low(m):
    """The low end of a displayed range, as a signed number. `L64` is -64."""
    left, plain = m.group(1), m.group(2)
    return -int(left) if left is not None else int(plain)


# The same continuation lines also say how a value is meant to be *read*: an enumeration's member
# names, a decimal scale, a unit. `Scene Tempo (500 - 30000)` shown as `5.00 - 300.00` is a
# hundredth of a BPM; `Zone Mono/Poly (0 - 2)` shown as `MONO, POLY, TONE` is an enumeration.
SCALED = re.compile(r"^[\s|]*(\d+)\.(\d+)\s*-\s*(\d+)\.(\d+)\s*(\[[^\]]*\])?\s*\|?\s*$")
UNIT = re.compile(r"\[([^\]]+)\]")
NUMERIC_RANGE_HEAD = re.compile(r"^[\s|]*[+-]?\d+\s*-\s*[+-]?\d+\s*,")


def last_column(line):
    """The description column of a table row, without the gutter.

    A continuation row is `|  |  |  MONO, POLY, TONE |` -- blank address and bit-pattern columns,
    then the text. Taking the last non-empty pipe-separated field gets it whatever the widths,
    which vary line to line in the extracted text.

    Rules and the `:` that elides a Reserved run carry no text; without discarding them the block's
    closing border becomes a label on its last field.
    """
    parts = [p for p in line.split("|") if p.strip()]
    if not parts:
        return ""
    text = parts[-1].strip()
    return "" if not text.strip("-+: ") else text


def display_of(text, span):
    """`(Display, unit)` for a field, from its continuation lines joined into `text`.

    Returns a plain number unless the document is unambiguous, and an enumeration only when the
    label count matches the declared range exactly — which rejects both a misparse and a range
    that merely reserves its top value, like `Zone Portamento Time` shown as `0 - 127, TONE`.
    """
    unit_m = UNIT.search(text)
    unit = unit_m.group(1) if unit_m else ""
    body = UNIT.sub("", text).strip().strip("|").strip()
    lo, hi = span

    if unit == "ASCII":
        return "Display::Ascii", ""
    if DISPLAY.match(text) and "L" in body.split("-")[0] and body.rstrip().endswith("R"):
        return "Display::Pan", ""
    m = SCALED.match(text)
    if m:
        whole, frac = m.group(1), m.group(2)
        shown = float(f"{whole}.{frac}")
        if shown:
            divisor = round(lo / shown)
            if divisor > 1:
                return f"Display::Scaled {{ divisor: {divisor}, decimals: {len(frac)} }}", unit
    if "," in body and not NUMERIC_RANGE_HEAD.match(body):
        labels = [t.strip() for t in body.split(",") if t.strip()]
        if len(labels) == hi - lo + 1 and lo == 0:
            joined = ", ".join(f'"{l}"' for l in labels)
            return f"Display::Enum(&[{joined}])", unit
    return "Display::Number", unit


def addr7(hi, lo):
    """A Roland address is 7 bits per byte."""
    return int(hi, 16) * 128 + int(lo, 16)


def sections(text):
    """Split the document into `* [Block Name]` sections ending at their Total Size."""
    cur, out = None, {}
    for line in text.splitlines():
        m = HEADING.match(line)
        if m:
            cur = m.group(1)
            out.setdefault(cur, {"lines": [], "total": None})
            continue
        if cur is None:
            continue
        t = TOTAL.match(line)
        if t:
            a, b, c, d = (int(x, 16) for x in t.groups())
            out[cur]["total"] = ((a * 128 + b) * 128 + c) * 128 + d
            cur = None
            continue
        out[cur]["lines"].append(line)
    return out


def fields(lines, declared):
    """The block's fields as (addr, nibbles, name, bias, display, unit).

    A `#` in the offset column opens a multi-address field whose description lands on its last
    line; the lines after that carry its displayed range or enumeration, and an enumeration can run
    over several of them. Roland elides runs of Reserved with a `:` row, so any gap in the address
    sequence, and anything between the last field and the declared size, is Reserved.
    """
    raw, pending = [], None

    def entry(addr, n, desc, span):
        return {"addr": addr, "n": n, "desc": desc, "span": span, "cont": []}

    for line in lines:
        m = LINE.match(line)
        if not m:
            if raw:
                raw[-1]["cont"].append(line)
            continue
        hash_, hi, lo, _bits, desc = m.groups()
        addr = addr7(hi, lo)
        rng = RANGE.search(desc)
        span = (int(rng.group(1)), int(rng.group(2).replace("+", ""))) if rng else (0, 0)
        desc = desc.split("(")[0].strip().rstrip("|").strip()

        if hash_:
            if pending:
                raw.append(entry(pending["addr"], pending["n"], "Reserved", (0, 0)))
            pending = entry(addr, 1, desc, span)
            continue
        if pending is not None:
            if addr == pending["addr"] + pending["n"]:
                pending["n"] += 1
                pending["desc"], pending["span"] = desc, span
                if desc:
                    raw.append(pending)
                    pending = None
                continue
            pending["desc"] = "Reserved"
            raw.append(pending)
            pending = None
        raw.append(entry(addr, 1, desc or "Reserved", span))
    if pending:
        pending["desc"] = pending["desc"] or "Reserved"
        raw.append(pending)

    for e in raw:
        text = " ".join(c for c in (last_column(l) for l in e["cont"]) if c)
        lo_store, _ = e["span"]
        e["bias"] = 0
        for line in e["cont"]:
            d = DISPLAY.match(line)
            if d:
                lo_show = display_low(d)
                # Only a *negative* displayed low means a field stored zero-centred. A display
                # that merely counts from 1 -- `Receive Channel (0 - 15)` shown as `1 - 16` -- is
                # a label for the player, and biasing by it would corrupt the wire value.
                if lo_show < 0:
                    e["bias"] = lo_store - lo_show
                break
        if e["desc"] == "Reserved":
            e["display"], e["unit"] = "Display::Number", ""
        else:
            e["display"], e["unit"] = display_of(text, e["span"])

    out, at = [], 0
    for e in raw:
        while at < e["addr"]:
            out.append((at, 1, "Reserved", 0, "Display::Number", ""))
            at += 1
        out.append((e["addr"], e["n"], e["desc"], e["bias"], e["display"], e["unit"]))
        at = e["addr"] + e["n"]
    while at < declared:
        out.append((at, 1, "Reserved", 0, "Display::Number", ""))
        at += 1
    return out


def ident(desc, used):
    """A stable Rust-side id for a field."""
    s = re.sub(r"[^A-Za-z0-9]+", "_", desc).strip("_")
    s = s or "reserved"
    if s[0].isdigit():
        s = "f_" + s
    base, n = s, 2
    while s in used:
        s, n = f"{base}_{n}", n + 1
    used.add(s)
    return s


def pack(fs):
    """Assign file offsets: a multi-nibble field is one little-endian integer, aligned to its
    own width, exactly as `docs/FORMAT.md` records for the tone table."""
    out, off = [], 0
    for a, n, d, bias, disp, unit in fs:
        width = 1 if n == 1 else n // 2
        if width > 1 and off % 2:
            off += 1
        out.append((off, width, a, n, d, bias, disp, unit))
        off += width
    return out, off


def main():
    text = sys.stdin.read()
    secs = sections(text)
    used_names = set()
    out = [
        "//! Scene parameter table: file bytes <-> SysEx addresses.",
        "//!",
        "//! Generated by `tools/gen_scene_params.py` from Roland's FANTOM EX MIDI",
        "//! Implementation; do not edit by hand.",
        "",
        "use super::{Block, Display, Instance, Param};",
        "",
    ]

    packed = {}
    for name, rust, _sx, _count, _stride in SCENE_BLOCKS:
        if name not in secs:
            sys.exit(f"block [{name}] not found in the MIDI Implementation text")
        sec = secs[name]
        fs = fields(sec["lines"], sec["total"])
        rows, nbytes = pack(fs)
        _base, file_len = PLACEMENT[rust]
        # Trailing Reserved may be unstored, but a real field must never fall off the end.
        last = max((r[0] + r[1] for r in rows if r[4] != "Reserved"), default=0)
        if last > file_len:
            sys.exit(f"[{name}] real field ends at {last}, past its {file_len}-byte slot")
        rows = [r for r in rows if r[0] + r[1] <= file_len]
        packed[rust] = (rows, file_len, sec["total"])

        ids = set()
        out.append(f"static {rust}_PARAMS: &[Param] = &[")
        for off, width, a, n, d, bias, disp, unit in rows:
            reserved = str(d == "Reserved").lower()
            out.append(
                f'    Param {{ id: "{ident(d, ids)}", byte_offset: {off}, '
                f"len_bytes: {width}, sysex_offset: {a}, "
                f"len_sysex: {1 if n == 1 else n}, bias: {bias}, "
                f"reserved: {reserved}, display: {disp}, "
                f'unit: "{unit}" }},'
            )
        out += ["];", ""]
        out += [
            f"pub static {rust}: Block = Block {{",
            f'    name: "{name}",',
            f"    byte_len: {file_len},",
            f'    sysex_len: {sec["total"]},',
            f"    params: {rust}_PARAMS,",
            "};",
            "",
        ]
        used_names.add(rust)

    out.append(f"/// A scene record ({RECORD_LEN} bytes), block by block.")
    out.append("///")
    out.append("/// Instances are in file order. The 16-fold tables are the zone parameters:")
    out.append("/// `Scene Zone` carries the tone reference and mix, `Zone EQ` the three-band EQ,")
    out.append("/// and `Zone Control` the keyboard split, velocity range and transpose.")
    out.append("pub static SCENE: &[Instance] = &[")
    at = 0
    for name, rust, sx, count, stride in SCENE_BLOCKS:
        base, file_len = PLACEMENT[rust]
        if base != at:
            sys.exit(f"[{name}] starts at {base:#x}, but the previous block ends at {at:#x}")
        for i in range(count):
            bo = base + i * file_len
            hi, mid, lo = sx
            mid += i * stride
            out.append(
                f"    Instance {{ block: &{rust}, byte_offset: {bo}, "
                f"sysex_offset: [0x{hi:02x}, 0x{mid:02x}, 0x{lo:02x}] }},"
            )
        at = base + count * file_len
    if at != RECORD_LEN:
        sys.exit(f"blocks cover {at} bytes, but a scene record is {RECORD_LEN}")
    out += ["];", ""]

    out += [
        f"/// On-disk length of one scene record in a `PRFa` area.",
        f"pub const RECORD_LEN: usize = {RECORD_LEN};",
        "",
    ]
    print("\n".join(out))


if __name__ == "__main__":
    main()
