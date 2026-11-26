// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 21:04:22.409695
//+ Revised: 	2026/11/24 16:26:03.051397
// -----------------------------------------------------------------------------

//! The short reasons a failed request is shown with, from every kind of failure ureq reports.

use avalio_lite::update::release::{CheckError, NetworkError};
use std::io;

fn failed(kind: io::ErrorKind) -> io::Error {
    io::Error::new(kind, "for the test")
}

#[test]
fn every_failed_call_has_its_reason() {
    let calls = [
        (ureq::Error::Timeout(ureq::Timeout::Global), NetworkError::TimedOut),
        (ureq::Error::HostNotFound, NetworkError::NoServer),
        (ureq::Error::BodyExceedsLimit(9), NetworkError::TooLarge),
        (ureq::Error::LargeResponseHeader(9, 8), NetworkError::TooLarge),
        (ureq::Error::RequireHttpsOnly("http://x".into()), NetworkError::NotHttps),
        (ureq::Error::TlsRequired, NetworkError::NotHttps),
        (ureq::Error::StatusCode(503), NetworkError::Status(503)),
        (ureq::Error::Tls("bad record"), NetworkError::Tls),
        (ureq::Error::Io(failed(io::ErrorKind::TimedOut)), NetworkError::TimedOut),
        (ureq::Error::Io(failed(io::ErrorKind::ConnectionRefused)), NetworkError::ConnectionFailed),
        (ureq::Error::ConnectionFailed, NetworkError::ConnectionFailed),
        (ureq::Error::RedirectFailed, NetworkError::ConnectionFailed),
        (ureq::Error::BadUri("certificate".into()), NetworkError::Tls),
        (ureq::Error::Other("TLS handshake refused".into()), NetworkError::Tls),
    ];
    for (error, expected) in calls {
        assert_eq!(NetworkError::from_ureq(&error), expected, "{error:?}");
    }
}

#[test]
fn a_failed_read_is_told_apart_by_its_kind_and_by_what_ureq_wrapped_in_it() {
    assert_eq!(NetworkError::from_io(&failed(io::ErrorKind::WouldBlock)), NetworkError::TimedOut);
    assert_eq!(NetworkError::from_io(&failed(io::ErrorKind::ConnectionReset)), NetworkError::ConnectionFailed);
    let wrapped = io::Error::other(ureq::Error::BodyExceedsLimit(9));
    assert_eq!(NetworkError::from_io(&wrapped), NetworkError::TooLarge);
}

#[test]
fn each_reason_reads_as_a_short_phrase_for_the_log() {
    let phrases = [
        (NetworkError::TimedOut, "the request timed out"),
        (NetworkError::NoServer, "the server couldn't be found"),
        (NetworkError::ConnectionFailed, "the connection failed"),
        (NetworkError::TooLarge, "the answer was too large"),
        (NetworkError::NotHttps, "the server didn't use HTTPS"),
        (NetworkError::Tls, "the secure connection failed"),
        (NetworkError::Status(404), "the server answered 404"),
    ];
    for (error, phrase) in phrases {
        assert_eq!(error.to_string(), phrase);
        assert_eq!(CheckError::Network(error).to_string(), phrase);
    }
}
