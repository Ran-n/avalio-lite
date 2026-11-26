// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.917639
//+ Revised: 	2026/11/24 17:54:11.752731
// -----------------------------------------------------------------------------

//! The update check and the signed in-app update: the forges' answers, the minisign signatures, the bridge
//! from 0.6.0 and 0.10.0, the signing tool, the download and installing. One module per topic.

mod bridge;
mod check;
#[path = "../common/mod.rs"]
mod common;
mod download;
mod forge_answers;
mod install;
mod network_errors;
mod overrides;
mod signatures;
mod signing_tool;
mod support;
mod targets;
