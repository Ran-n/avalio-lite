// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:10.659616
//+ Revised: 	2026/11/23 11:49:10.659616
// -----------------------------------------------------------------------------

//! One cell of the workbook as the text typing it would store: numbers, dates, durations and
//! booleans, the same from .xlsx, .xls and .ods.

use calamine::Data;

/// A cell as the text typing it would store: numbers without a locale (`7.5`, Rust's shortest
/// form of the number Excel shows), text trimmed, `TRUE`/`FALSE`, dates as `YYYY-MM-DD`,
/// durations and times of day as `H:MM:SS`, the same from .xlsx, .xls and .ods. `None` for
/// empty and error cells.
pub fn cell_text(data: &Data) -> Option<String> {
    let text = match data {
        Data::Empty | Data::Error(_) => return None,
        Data::Int(number) => number.to_string(),
        Data::Float(number) if number.is_finite() => without_float_noise(*number).to_string(),
        Data::Float(_) => return None,
        Data::String(text) => text.trim().to_owned(),
        Data::Bool(value) => (if *value { "TRUE" } else { "FALSE" }).to_owned(),
        // a duration, or a time of day with no date (Excel counts it from day 0)
        Data::DateTime(moment) if moment.is_duration() || moment.as_f64().abs() < 1.0 => {
            clock((moment.as_f64() * SECONDS_PER_DAY).round())
        }
        Data::DateTime(moment) => {
            let (year, month, day, ..) = moment.to_ymd_hms_milli();
            format!("{year:04}-{month:02}-{day:02}")
        }
        // .ods: "2011-03-09" or "2011-03-09T08:30:00"
        Data::DateTimeIso(text) => text.split('T').next().unwrap_or_default().to_owned(),
        // .ods: "PT01H30M05S"; anything else is kept as written
        Data::DurationIso(text) => iso_seconds(text).map_or_else(|| text.clone(), clock),
    };
    (!text.is_empty()).then_some(text)
}

const SECONDS_PER_DAY: f64 = 86_400.0;

/// What Excel itself shows of a number: the digits past these are binary noise.
const SIGNIFICANT_DIGITS: usize = 15;

/// `number` to Excel's 15 significant digits, so a formula's 7.2999999999999989 is 7.3.
pub fn without_float_noise(number: f64) -> f64 {
    format!("{number:.*e}", SIGNIFICANT_DIGITS - 1).parse().unwrap_or(number)
}

/// `seconds` (whole) as hours, minutes and seconds: 5405 is "1:30:05", -5405 is "-1:30:05".
pub fn clock(seconds: f64) -> String {
    let sign = if seconds < 0.0 { "-" } else { "" };
    let seconds = seconds.abs() as i64;
    format!("{sign}{}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
}

/// The length of a time duration as .ods files write it, `PT01H30M05S`, in whole seconds.
pub fn iso_seconds(text: &str) -> Option<f64> {
    let (hours, rest) = text.strip_prefix("PT")?.split_once('H')?;
    let (minutes, rest) = rest.split_once('M')?;
    let seconds = rest.strip_suffix('S')?;
    let number = |text: &str| text.parse::<f64>().ok();
    Some((number(hours)? * 3600.0 + number(minutes)? * 60.0 + number(seconds)?).round())
}
