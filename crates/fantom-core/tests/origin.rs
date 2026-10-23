//! `DIFa` stamps in files the instrument wrote.

#[allow(dead_code)]
mod support;
use support::private;

use fantom_core::origin::{self, Origin};

/// A FANTOM-6 backup decodes to the FANTOM-6 product code, and re-encodes to the stored bytes.
#[test]
fn a_backup_names_the_instrument_that_wrote_it() {
    let Some(raw) = private("TONEMAP4/FANTOM.SVD") else {
        return;
    };
    let Some(Origin::Stamped(stamp)) = origin::read(&raw).unwrap() else {
        panic!("expected a stamped DIFa");
    };
    assert_eq!(stamp.product_str(), "420031");
    assert_eq!(stamp.serial_str().len(), 7);

    let svd = fantom_core::container::Svd::parse(&raw).unwrap();
    let area = svd.area(b"DIFa").unwrap();
    let bytes = svd.area_bytes(&raw, area).unwrap();
    assert_eq!(&bytes[16..48], &stamp.encode());
}

/// Every SVZ the instrument writes carries a blank stamp.
#[test]
fn an_svz_export_is_blank() {
    let Some(raw) = private("EXPORT_Z-Core.svz") else {
        return;
    };
    assert_eq!(origin::read(&raw).unwrap(), Some(Origin::Blank));
}

/// Older backups carry a blank stamp too.
#[test]
fn an_older_backup_is_blank() {
    let Some(raw) = private("backup/ROLAND/SOUND/PRISMA/FANTOM.SVD") else {
        return;
    };
    assert_eq!(origin::read(&raw).unwrap(), Some(Origin::Blank));
}
