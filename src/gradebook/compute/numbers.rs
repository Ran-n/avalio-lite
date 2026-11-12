// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:39:46.297306
//+ Revised: 	2026/11/12 14:39:52.174842
// -----------------------------------------------------------------------------

//! Reading and showing marks: decimal commas, rounding, and the float noise in between.

pub const DISPLAY_DECIMALS: u8 = 2;

const FLOAT_SNAP: f64 = 1e6;

/// Accepts both "7.5" and "7,5": digits with at most one decimal separator, and a minus sign in
/// front. Anything else ("1e2", "+5", "inf") isn't a number a teacher types.
pub fn parse_number(text: &str) -> Option<f64> {
    let text = text.trim();
    let digits = text.strip_prefix('-').unwrap_or(text);
    let mut parts = digits.splitn(2, ['.', ',']);
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next().unwrap_or_default();
    let all_digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    if whole.len() + fraction.len() == 0 || !all_digits(whole) || !all_digits(fraction) {
        return None;
    }
    let number: f64 = text.replace(',', ".").parse().ok()?;
    number.is_finite().then_some(number)
}

/// Up to two decimals, rounded half up like `round_to`, without trailing zeros: 8, 7.5, 6.25.
pub fn format_number(value: f64) -> String {
    let text = format!("{:.*}", usize::from(DISPLAY_DECIMALS), round_to(value, DISPLAY_DECIMALS));
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" { "0".to_owned() } else { trimmed.to_owned() }
}

/// Rounds half away from zero. Float noise is snapped off first, so 7.45 becomes 7.5
/// rather than 7.4 (7.45 × 10 is 74.4999… in binary).
pub fn round_to(value: f64, decimals: u8) -> f64 {
    let factor = 10f64.powi(i32::from(decimals));
    ((value * factor * FLOAT_SNAP).round() / FLOAT_SNAP).round() / factor
}
