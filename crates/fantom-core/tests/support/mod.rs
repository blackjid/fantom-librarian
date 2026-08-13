//! Opening the two tiers of test fixture, with the difference between them made structural.
//!
//! `fixtures/` is committed and `fixtures-local/` is not, because the private corpus holds
//! instrument backups, Roland documentation and purchased pack content that cannot be published
//! (see `fixtures/README.md`). The two functions here differ in exactly one way, and it is the
//! important one: [`public`] fails when its file is missing, [`private`] skips.
//!
//! That asymmetry is deliberate. A test that skips when its data is absent reports `ok` having
//! asserted nothing — fine as a convenience for a fresh clone, disastrous as the only thing behind
//! a green CI badge. Anything reachable from [`public`] therefore *always* runs.

use std::path::{Path, PathBuf};

use fantom_core::container::Raw;

/// Set `FANTOM_FIXTURES=require` to turn a missing private fixture into a failure.
const REQUIRE_VAR: &str = "FANTOM_FIXTURES";

/// Set `FANTOM_FIXTURES_DIR` to keep the private corpus outside the repository.
const DIR_VAR: &str = "FANTOM_FIXTURES_DIR";

/// Default location of the private corpus, beside the public one.
const PRIVATE_DIR: &str = "fixtures-local";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Open a committed fixture. Panics when it is missing.
///
/// These files are in the repository, so their absence is a broken checkout rather than a reason
/// to skip — and skipping is what would let CI pass without testing anything.
pub fn public(relative: &str) -> Raw {
    let path = repo_root().join("fixtures").join(relative);
    assert!(
        path.is_file(),
        "committed fixture missing: {}\n\
         This file is tracked in git; a checkout without it is broken. See fixtures/README.md.",
        path.display()
    );
    Raw::open(&path).expect("fixture is readable")
}

/// Open a fixture from the private corpus, or return `None` so the caller can skip.
///
/// Returns `None` when the corpus is absent, so a clone without it still runs green. Under
/// `FANTOM_FIXTURES=require` a missing file panics instead, which is how a machine that *has* the
/// corpus keeps these tests honest.
pub fn private(relative: &str) -> Option<Raw> {
    let path = private_root().join(relative);
    if !path.is_file() {
        assert!(
            !required(),
            "private fixture missing: {}\n\
             {REQUIRE_VAR}=require is set, so this is a failure rather than a skip.\n\
             Unset it, or point {DIR_VAR} at the corpus. See fixtures/README.md.",
            path.display()
        );
        eprintln!("skipping: {} not present", path.display());
        return None;
    }
    Some(Raw::open(&path).expect("fixture is readable"))
}

fn private_root() -> PathBuf {
    match std::env::var(DIR_VAR) {
        Ok(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => repo_root().join(PRIVATE_DIR),
    }
}

fn required() -> bool {
    std::env::var(REQUIRE_VAR).is_ok_and(|v| v == "require")
}
