//! Fixture source tree for the quoin-quire integration tests.
//!
//! Only TC-001 carries a trace marker. TC-002 deliberately carries none, so
//! the fixture has one backed row, one unbacked row, and a status value that
//! claims complete for a row nothing backs — the three coverage findings this
//! crate must surface.

#[trace("TC-001")]
#[test]
fn covers_the_round_trip() {
    let _ = 1;
}
