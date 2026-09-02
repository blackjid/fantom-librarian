#!/usr/bin/env python3
"""Build the bundled expansion catalogs out of captured bank pages.

    tools/gen_expansion_catalog.py sounds/*.tsv > crates/fantom-core/src/expansion_sounds.tsv

Every input is one bank page in the shape `dump-sounds` writes — `msb lsb pc number name
category` — so both sources of a page feed this: a SysEx sweep of an instrument that has the
expansion installed, or `pdftotext -layout <expansion sound list> | tools/gen_sound_list.py`.
Lines that are not a page row are ignored, which is what lets a captured console log be passed
straight in.

The output adds one column: the **product code** the page belongs to. That is the catalog's key,
because the contents of `EXZ007` are the same for everyone who owns it, while the address it
answers at is only what one instrument was observed doing.

Rows with no name are dropped — an uninstalled bank leaves the last name in place and a sweep
ends on a blank, so neither is a sound. A page at an address no product claims is dropped whole,
with a line on stderr: an unnamed product would be an invented one.
"""

import re
import sys

# Address to product, mirroring `fantom_core::model::ToneRef::bank`. Only the expansions are
# here; the base instrument's own banks are `crates/fantom-core/src/factory_sounds.tsv`.
# `expansions::tests::every_row_is_keyed_as_the_address_taxonomy_keys_it` pins the two together.
PRODUCTS = {
    (93, 1): "EXZ013", (93, 2): "EXZ005", (93, 3): "EXZ009",
    (93, 7): "EXZ007", (93, 8): "EXZ007", (93, 9): "EXZ007", (93, 10): "EXZ007",
    (93, 11): "EXZ008", (93, 12): "EXZ008", (93, 13): "EXZ008", (93, 14): "EXZ008",
    (93, 15): "EXZ012", (93, 16): "EXZ012", (93, 17): "EXZ012",
    (93, 19): "EXZ006", (93, 20): "EXZ006", (93, 21): "EXZ006", (93, 22): "EXZ006",
    (93, 23): "EXZ010", (93, 24): "EXZ014", (93, 26): "EXZ011", (93, 27): "EXZ015",
    (100, 64): "EXZ003", (100, 65): "EXZ004",
    (101, 64): "EXZ001", (101, 65): "EXZ002",
    (97, 64): "JP8", (97, 66): "JU106", (97, 68): "JX8P",
    (97, 70): "SH101", (97, 72): "JD800", (97, 79): "n/zyme",
    (103, 64): "M09X01",
    (105, 64): "EXSN01", (105, 65): "EXSN02", (105, 66): "EXSN03", (105, 67): "EXSN04",
}

ROW = re.compile(r"^(\d{1,3})\t(\d{1,3})\t(\d{1,3})\t(\d*)\t([^\t]*)(?:\t([^\t]*))?")


def rows(text):
    """Every `msb lsb pc number name category` row in one page, whatever else the file holds."""
    for line in text.splitlines():
        found = ROW.match(line)
        if found:
            yield found.groups(default="")


def main():
    catalog = {}
    for path in sys.argv[1:]:
        with open(path, encoding="utf-8") as page:
            kept = 0
            for msb, lsb, pc, number, name, category in rows(page.read()):
                product = PRODUCTS.get((int(msb), int(lsb)))
                if product is None:
                    print(f"{path}: no product owns {msb}/{lsb}", file=sys.stderr)
                    break
                if not name.strip():
                    continue
                key = (int(msb), int(lsb), int(pc))
                if key in catalog:
                    continue  # first page named an address wins, so a rerun is stable
                catalog[key] = (product, msb, lsb, pc, number, name.strip(), category.strip())
                kept += 1
            else:
                print(f"{path}: {kept} sounds", file=sys.stderr)

    print("product\tmsb\tlsb\tpc\tnumber\tname\tcategory")
    for row in sorted(catalog.values(), key=lambda r: (r[0], int(r[1]), int(r[2]), int(r[3]))):
        print("\t".join(row))


if __name__ == "__main__":
    main()
