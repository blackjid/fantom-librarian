#!/usr/bin/env python3
"""Extract the instrument's built-in sounds from a Roland sound-list PDF.

    pdftotext -layout FANTOM_SoundList.pdf - | tools/gen_sound_list.py > factory_sounds.tsv

Every bank in these PDFs prints the same way, whatever section it is under: the columns end with
`MSB LSB PC`, with the tone's name — and usually its bank and number — to their left and its
category to their right. Pages are two records wide, so a line can hold two of them.

Engine and bank names are deliberately *not* emitted: `fantom_core::model::ToneRef` already derives
both from the address, and one taxonomy is better than two that can disagree.
"""

import re
import sys

# Bank Select MSBs the FANTOM uses for sounds, from `fantom_core::model::ToneRef::tone_type`.
MSB = {86, 87, 89, 90, 91, 92, 93, 97, 100, 101, 103, 105, 107}
CATEGORY = re.compile(r"^(?:\d{1,2}:.+|Drums)$")
NUMBER = re.compile(r"^\d{1,4}$")
BANK = re.compile(r"^(?:PR-[A-Z]|CMN|PRST|USER|EXZ\d+|EXSN\d+|M\d\w+)$")


def rows(line):
    """Every `… name MSB LSB PC [category]` record on one line."""
    cells = re.split(r"\s{2,}", line.strip())
    for i in range(len(cells) - 2):
        head, msb, lsb, pc = cells[i - 1 : i] , cells[i], cells[i + 1], cells[i + 2]
        if not (NUMBER.match(msb) and NUMBER.match(lsb) and NUMBER.match(pc)):
            continue
        if int(msb) not in MSB or int(lsb) > 127 or int(pc) > 128:
            continue
        if not head or not head[0] or NUMBER.match(head[0]):
            continue  # the name is never a bare number
        name = head[0]
        number = cells[i - 2] if i >= 2 and NUMBER.match(cells[i - 2]) else ""
        category = ""
        if i + 3 < len(cells) and CATEGORY.match(cells[i + 3]):
            category = cells[i + 3]
        yield (msb, lsb, pc, number, name, category)


def main():
    seen = set()
    out = []
    for line in sys.stdin:
        for row in rows(line):
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
