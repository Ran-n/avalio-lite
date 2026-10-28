// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:46.536666
//+ Revised: 	2026/10/28 22:23:17.200355
// -----------------------------------------------------------------------------

use std::time::{Duration, Instant};

use avalio_lite::error::AppError;
use avalio_lite::lock::{AttemptLimiter, Pin, pins_match, validate_new_pin, validate_pin};

#[test]
fn pins_are_4_to_128_characters_of_any_kind() {
    let longest = "a".repeat(128);
    for good in ["1234", "00000000", "12a4", " 1234", "１２３４", "corvo azul 7!", "Uxía's pin", longest.as_str()]
    {
        assert!(validate_pin(good).is_ok(), "{good}");
    }
    let too_long = "a".repeat(129);
    for bad in ["123", "", "abc", "12\t34", "1234\n", too_long.as_str()] {
        assert!(matches!(validate_pin(bad), Err(AppError::InvalidPin)), "{bad:?}");
    }
}

#[test]
fn a_new_pin_must_be_typed_the_same_twice() {
    assert!(validate_new_pin("2468", "2468").is_ok());
    assert!(matches!(validate_new_pin("2468", "2469"), Err(AppError::PinMismatch)));
    assert!(matches!(validate_new_pin("12", "12"), Err(AppError::InvalidPin)), "format is checked first");
}

#[test]
fn guessing_is_slowed_down_after_five_wrong_pins() {
    let start = Instant::now();
    let mut limiter = AttemptLimiter::default();
    for _ in 0..4 {
        limiter.record_failure(start);
        assert_eq!(limiter.wait_remaining(start), None);
    }
    limiter.record_failure(start);
    assert_eq!(limiter.wait_remaining(start), Some(Duration::from_secs(30)));
    assert!(limiter.wait_remaining(start + Duration::from_secs(10)).is_some());
    assert_eq!(limiter.wait_remaining(start + Duration::from_secs(31)), None);

    limiter.record_failure(start + Duration::from_secs(31));
    assert!(limiter.wait_remaining(start + Duration::from_secs(31)).is_some(), "each further miss waits again");
    limiter.record_success();
    assert_eq!(limiter.wait_remaining(start + Duration::from_secs(31)), None);
}

proptest::proptest! {
    #[test]
    fn a_pin_is_valid_exactly_when_it_has_4_to_128_characters_and_no_control_ones(pin in r"(\PC|[\t\n\r]){0,140}") {
        let expected = (4..=128).contains(&pin.chars().count()) && !pin.chars().any(char::is_control);
        proptest::prop_assert_eq!(validate_pin(&pin).is_ok(), expected);
    }

    #[test]
    fn a_valid_pin_only_matches_itself(pin in "[0-9]{4,8}", other in "[0-9]{4,8}") {
        proptest::prop_assert!(validate_new_pin(&pin, &pin).is_ok());
        proptest::prop_assert_eq!(validate_new_pin(&pin, &other).is_ok(), pin == other);
    }

    #[test]
    fn pins_match_exactly_when_they_are_equal(pin in r"\PC{0,10}", other in r"\PC{0,10}") {
        proptest::prop_assert!(pins_match(&pin, &pin));
        proptest::prop_assert_eq!(pins_match(&pin, &other), pin == other);
    }
}

#[test]
fn a_pin_keeps_its_digits_but_never_prints_them() {
    let pin = Pin::new("583920");
    assert_eq!(pin.as_str(), "583920");
    assert_eq!(pin.clone().as_str(), "583920");
    let shown = format!("{pin:?} {:?}", Some(pin.clone()));
    assert!(!shown.contains("583920"), "{shown}");
}
