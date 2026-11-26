// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:37.570535
//+ Revised: 	2026/11/23 13:00:37.570535
// -----------------------------------------------------------------------------

//! Helpers shared by the state tests: adding pupils and counting them.

use avalio_lite::state::AppState;

pub(crate) fn pupil_count(state: &AppState) -> i64 {
    state.connection().unwrap().query_row("SELECT COUNT(*) FROM pupils", [], |row| row.get(0)).unwrap()
}

pub(crate) fn add_pupil(state: &AppState) {
    state.connection().unwrap().execute("INSERT INTO pupils (first_name) VALUES ('Ana Pérez')", []).unwrap();
}
