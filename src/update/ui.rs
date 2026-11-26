// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.893101
//+ Revised: 	2026/11/23 03:55:26.532687
// -----------------------------------------------------------------------------

//! The banner and Settings › Updates: timers, the check and download threads, and what to show.
//! Every rule is in the UI-free modules; this only schedules, starts and shows. Each concern is a
//! file under `ui/`: `updater` (the state), `wiring` (the callbacks), `schedule` (the timers and
//! checks), `installing` (the download and the install) and `view` (what is shown). The public
//! paths are re-exported here.

mod installing;
mod schedule;
mod updater;
mod view;
mod wiring;

pub use schedule::{after_start, automatic_check, start};
pub use updater::{QUIT_WAIT, Updater, Updates};
pub use view::show;
pub use wiring::wire;
