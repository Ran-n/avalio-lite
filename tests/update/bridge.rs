// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.458681
//+ Revised: 	2026/11/24 17:54:11.646168
// -----------------------------------------------------------------------------

//! The bridge from 0.6.0 to 0.10.0: the old file names and the first built-in key.

use crate::common::release_key;
use crate::common::releases::version;
use crate::support::{as_0_10_0, hash_of, stranger};
use avalio_lite::update::signature::{BUILT_IN_KEYS, SigError, bridge, public_key};
use avalio_lite::update::target::{Form, bridge_asset_name};

#[test]
fn the_bridge_key_is_the_first_built_in_key_and_the_one_0_10_0_trusts() {
    assert_eq!(bridge::bridge_key(), public_key(BUILT_IN_KEYS[0]).unwrap());
    assert_eq!(bridge::bridge_key(), as_0_10_0::public_key(as_0_10_0::PUBLIC_KEY));
    assert_eq!(bridge::BRIDGE_KEY, as_0_10_0::PUBLIC_KEY);
}

/// What the signing tool writes for the old-named files passes 0.10.0's own check, and the names
/// are the ones it asks for.
#[test]
fn bridge_signatures_pass_the_check_0_10_0_runs() {
    let key = release_key();
    let version = version("0.11.0");
    for (form, kind) in [(Form::Portable, "portable"), (Form::Setup, "setup")] {
        let name = bridge_asset_name(&version, form);
        assert_eq!(name, as_0_10_0::asset_name(&version, kind));
        let hash = hash_of(b"the new version");
        let text = bridge::signature_text(&bridge::sign(&key, &name, &hash), &name);
        assert!(as_0_10_0::verify(&key.verifying_key(), &name, &hash, &text));
        assert!(!as_0_10_0::verify(&key.verifying_key(), &name, &hash_of(b"other bytes"), &text));
        assert!(!as_0_10_0::verify(&stranger().verifying_key(), &name, &hash, &text));
        assert_eq!(bridge::verify(&key.verifying_key(), &name, &hash, &text), Ok(()));
        let renamed = bridge::verify(&key.verifying_key(), "another.exe", &hash, &text);
        assert_eq!(renamed, Err(SigError::NotSigned), "it signs its own name only");
    }
}
