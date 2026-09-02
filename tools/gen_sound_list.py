#!/usr/bin/env python3
"""Extract named built-in sounds and scenes from a Roland sound-list PDF.

    pdftotext -layout FANTOM_SoundList.pdf - | tools/gen_sound_list.py > factory_sounds.tsv

The bundled table is both of Roland's lists, minus the ZEN-Core presets `preset_tones.tsv`
already holds. The ACB JUPITER-8 bank is only in the FANTOM EX one, so leaving it out silently
drops 44 sounds:

    cat base.txt ex.txt | tools/gen_sound_list.py | awk -F'\t' 'NR==1 || $1!=87' \
        > crates/fantom-core/src/factory_sounds.tsv

A bank Roland never printed — the VTW presets are one — has no PDF to read. Pass the TSV a
`dump-sounds` sweep wrote as an argument and it is merged in:

    tools/gen_sound_list.py 91-65.tsv < base.txt

Every bank in these PDFs prints the same way, whatever section it is under: the columns end with
`MSB LSB PC`, with the tone's name — and usually its bank and number — to their left and its
category to their right. Pages are two records wide, so a line can hold two of them.

Two records per bank do not print that way, and both are recovered here rather than lost:

* where a column header interrupts the table, `pdftotext` breaks the record into one cell per
  line, labels and values alternating;
* where a name is too long for its column, it wraps onto the lines above and below, leaving the
  record's own line with no name at all.

Engine and bank names are deliberately *not* emitted: `fantom_core::model::ToneRef` already derives
both from the address, and one taxonomy is better than two that can disagree.
"""

import re
import sys

# Bank Select MSBs the FANTOM uses for sounds, plus 85 for factory-scene names.
MSB = {85, 86, 87, 89, 90, 91, 92, 93, 97, 100, 101, 103, 105, 107}
CATEGORY = re.compile(r"^(?:\d{1,2}:.+|Drums)$")
NUMBER = re.compile(r"^\d{1,4}$")
BANK = re.compile(r"^(?:PR-[A-Z]|CMN|PRST|USER|EXZ\d+|EXSN\d+|M\d\w+)$")
# Column labels, which a broken record carries inline with its values.
LABELS = {"Bank", "No.", "Tone Name", "MSB", "LSB", "PC", "Category", "Memo"}


def rows(cells):
    """Every `… name MSB LSB PC [category]` record in one line's cells."""
    for i in range(len(cells) - 2):
        head, msb, lsb, pc = cells[i - 1 : i], cells[i], cells[i + 1], cells[i + 2]
        if not (NUMBER.match(msb) and NUMBER.match(lsb) and NUMBER.match(pc)):
            continue
        if int(msb) not in MSB or int(lsb) > 127 or int(pc) > 128:
            continue
        if not head or not head[0]:
            continue
        # A name can be a bare number — `2080` is a PR-E tone — but only where the No. and Bank
        # columns are to its left. Otherwise this is the No. column and the record has no name of
        # its own, which the wrapped-name case in `unwrapped` is what fills in.
        if NUMBER.match(head[0]) and not (
            i >= 3 and NUMBER.match(cells[i - 2]) and BANK.match(cells[i - 3])
        ):
            continue
        name = head[0]
        number = cells[i - 2] if i >= 2 and NUMBER.match(cells[i - 2]) else ""
        category = ""
        if i + 3 < len(cells) and CATEGORY.match(cells[i + 3]):
            category = cells[i + 3]
        yield (msb, lsb, pc, number, name, category)


def unwrapped(lines):
    """The lines to scan, with the two records that do not print as one line put back together.

    A run of one-cell lines is a broken record when it carries the column labels; it is rebuilt by
    dropping them. A record whose own line has a number where its name should be took the name
    onto the one-cell lines each side of it; those are joined.
    """
    cells = [re.split(r"\s{2,}", line.strip()) for line in lines]
    runs = {}  # index of a one-cell run's first line -> its values
    start = None
    for i, line in enumerate(cells + [[]]):
        if len(line) == 1 and line[0]:
            start = i if start is None else start
            runs.setdefault(start, []).append(line[0])
        else:
            start = None

    for i, line in enumerate(cells):
        run = runs.get(i)
        if run is not None:
            if {"MSB", "LSB", "PC"} <= set(run):
                yield [value for value in run if value not in LABELS]
            continue
        if len(line) == 1:
            continue  # already yielded, or prose
        yield line
        # A record with no name of its own: the lines each side of it hold the halves.
        if len(line) >= 4 and NUMBER.match(line[0]) and not list(rows(line)):
            before = next((v for k, v in runs.items() if k + len(v) == i), None)
            after = runs.get(i + 1)
            if before and after and not NUMBER.match(before[-1]):
                yield [line[0], f"{before[-1]} {after[0]}"] + line[1:]


def swept(paths):
    """Rows from a `dump-sounds` sweep, which already writes the output shape."""
    for path in paths:
        with open(path, encoding="utf-8") as page:
            for line in page:
                cells = line.rstrip("\n").split("\t")
                if len(cells) >= 5 and all(c.isdigit() for c in cells[:3]) and cells[4].strip():
                    yield (cells[0], cells[1], cells[2], cells[3], cells[4].strip(),
                           cells[5].strip() if len(cells) > 5 else "")


def main():
    seen = set()
    out = []
    printed = (row for cells in unwrapped(sys.stdin) for row in rows(cells))
    for row in list(printed) + list(swept(sys.argv[1:])):
        key = row[:3]
        if key in seen:
            continue
        seen.add(key)
        out.append(row)
    out.sort(key=lambda r: (int(r[0]), int(r[1]), int(r[2])))
    print("msb\tlsb\tpc\tnumber\tname\tcategory")
    for row in out:
        print("\t".join(row))


if __name__ == "__main__":
    main()
