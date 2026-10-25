//! `SYSa` by name: reads the public fixtures.
//!
//! Parity with `fixtures-local/tools/sysa.py` over the private corpus is in `fantom-cli/tests/system.rs`.

use fantom_core::sysa::read_sysa;

#[allow(dead_code)]
mod support;
use support::public;

#[test]
fn a_public_fixture_reads_by_name() {
    let sysa = read_sysa(&public("tests/TEST 1/FANTOM.SVD")).unwrap();
    assert_eq!(sysa.get("SC:TUNE"), Some(0));
    assert_eq!(sysa.get("SC:USB_MIDI_DRV"), Some(0));
    assert_eq!(sysa.get("SC:CLOCK_YEAR"), Some(19));
    assert_eq!(sysa.get("IN_REV:Reverb Type"), Some(1));
    assert_eq!(sysa.get("nope"), None);

    let fields = sysa.fields();
    let find = |id: &str| fields.iter().find(|f| f.param.id == id).unwrap();
    assert_eq!(sysa.describe(find("SC:USB_MIDI_DRV")), "0 VENDOR");
    assert_eq!(sysa.describe(find("SC:CLOCK_YEAR")), "19 (2019)");
    // Reverb parameters are named by the current reverb type.
    assert!(sysa
        .describe(find("IN_REV:Reverb Parameter 1"))
        .contains("INTEGRA-7 Reverb: Type"));
    assert!(sysa.at_init(find("SC:TUNE")));
    assert!(!sysa.at_init(find("SC:CLOCK_YEAR")));
}

#[test]
fn identical_records_do_not_differ() {
    let sysa = read_sysa(&public("tests/TEST 1/FANTOM.SVD")).unwrap();
    assert!(sysa.diff(&sysa.clone()).is_empty());
}

#[test]
fn a_file_without_sysa_is_an_error() {
    let raw = fantom_core::container::Raw::from_bytes(vec![0; 64]);
    assert!(read_sysa(&raw).is_err());
}
