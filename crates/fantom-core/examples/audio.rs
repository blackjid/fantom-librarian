//! Throwaway: is the carried audio real, and do these samples' SMPd flags match the ones we proved?
use fantom_core::container::{Raw, Svd};

fn u32at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// Walk a backup's USDa: 8-byte {slot, offset} pairs, then SMPd sections.
fn backup_sections(raw: &Raw, svd: &Svd) -> Vec<(usize, usize, usize)> {
    let area = svd.area(b"USDa").unwrap();
    let bytes = svd.area_bytes(raw, area).unwrap();
    let body = &bytes[16..];
    let mut out = Vec::new();
    let mut at = 0;
    loop {
        let slot = u32at(body, at);
        let offset = u32at(body, at + 4) as usize;
        if slot == u32::MAX {
            break;
        }
        let size = u32at(body, offset + 0x08) as usize;
        out.push((slot as usize, offset, size));
        at += 8;
    }
    out
}

fn main() {
    for (label, path, slots) in [
        (
            "TESTv1 (today, the tones that will not sound)",
            "fixtures-local/TESTv1/FANTOM.SVD",
            vec![28usize, 29, 30, 5, 34, 54, 55],
        ),
        (
            "T8 backup (the samples we reproduced byte for byte)",
            "fixtures-local/hwtest_back/T8_MSMP_BACKUP/FANTOM.SVD",
            vec![2000, 2001, 2002, 2004, 2017],
        ),
    ] {
        let Ok(raw) = Raw::open(path) else { continue };
        let svd = Svd::parse(&raw).unwrap();
        let area = svd.area(b"USDa").unwrap();
        let bytes = svd.area_bytes(&raw, area).unwrap();
        let body = &bytes[16..];
        println!("\n{label}");
        for (slot, offset, size) in backup_sections(&raw, &svd) {
            if !slots.contains(&slot) {
                continue;
            }
            // A section spans size + 64 bytes, of which the first 0x80 is header — so the audio
            // runs to offset + size + 64, not offset + 0x80 + size.
            let audio = &body[offset + 0x80..(offset + size + 64).min(body.len())];
            let nonzero = audio.iter().filter(|&&b| b != 0).count();
            let first = audio.iter().position(|&b| b != 0);
            let last = audio.iter().rposition(|&b| b != 0);
            println!(
                "  slot {:>5} (panel {:>5}): flags {:#010x} words {:>9} size {:>9}  name {:?}\n      \
                 non-zero {nonzero} of {} (first {first:?}, last {last:?})",
                slot,
                slot + 1,
                u32at(body, offset + 0x04),
                u32at(body, offset + 0x0c),
                size,
                String::from_utf8_lossy(&body[offset + 0x10..offset + 0x20]).trim().to_string(),
                audio.len(),
            );
        }
    }

    // How much of today's sample bank still holds audio.
    let raw = Raw::open("fixtures-local/TESTv1/FANTOM.SVD").unwrap();
    let svd = Svd::parse(&raw).unwrap();
    let area = svd.area(b"USDa").unwrap();
    let bytes = svd.area_bytes(&raw, area).unwrap();
    let body = &bytes[16..];
    let mut silent = Vec::new();
    let mut sounding = Vec::new();
    for (slot, offset, size) in backup_sections(&raw, &svd) {
        let audio = &body[offset + 0x80..(offset + size + 64).min(body.len())];
        if audio.iter().all(|&b| b == 0) {
            silent.push(slot + 1);
        } else {
            sounding.push(slot + 1);
        }
    }
    println!(
        "\n{} slots hold audio, {} are empty",
        sounding.len(),
        silent.len()
    );
    println!("empty (panel numbers): {silent:?}");
    println!("holding audio: {sounding:?}");
}
