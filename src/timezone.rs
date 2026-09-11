use std::env;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Flag,
    JatabagTz,
    Tz,
    OperatingSystem,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Source::Flag => "--timezone",
            Source::JatabagTz => "JATABAG_TZ",
            Source::Tz => "TZ",
            Source::OperatingSystem => "the operating system",
        })
    }
}

pub struct Timezone {
    name: String,
    source: Source,
}

impl Timezone {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source(&self) -> Source {
        self.source
    }
}

pub enum Error {
    Unrecognized { name: String, source: Source },
    Undetermined,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unrecognized { name, source } => write!(
                f,
                "{source} names the timezone {name:?}, which is not one this system knows"
            ),
            Error::Undetermined => f.write_str(
                "no timezone found: the operating system reports none, and neither \
                 JATABAG_TZ nor TZ is set. Pass --timezone to name one",
            ),
        }
    }
}

pub fn resolve(flag: Option<&str>) -> Result<Timezone, Error> {
    pick([
        (Source::Flag, flag.map(str::to_owned)),
        (Source::JatabagTz, from_env("JATABAG_TZ")),
        (Source::Tz, from_env("TZ")),
        (Source::OperatingSystem, iana_time_zone::get_timezone().ok()),
    ])
}

fn pick(candidates: impl IntoIterator<Item = (Source, Option<String>)>) -> Result<Timezone, Error> {
    for (source, value) in candidates {
        let Some(value) = value.filter(|value| !value.trim().is_empty()) else {
            continue;
        };

        let name = value.trim().trim_start_matches(':').to_owned();

        return if recognized(&name, source) {
            Ok(Timezone { name, source })
        } else {
            Err(Error::Unrecognized { name, source })
        };
    }

    Err(Error::Undetermined)
}

fn recognized(name: &str, source: Source) -> bool {
    jiff::tz::TimeZone::get(name).is_ok()
        || (source == Source::Tz && jiff::tz::TimeZone::posix(name).is_ok())
}

fn from_env(variable: &str) -> Option<String> {
    env::var(variable).ok()
}

#[cfg(test)]
mod pick {
    use super::*;

    fn candidate(source: Source, value: &str) -> (Source, Option<String>) {
        (source, Some(value.to_owned()))
    }

    #[test]
    fn takes_the_first_source_that_has_a_value() {
        let picked = pick([
            (Source::Flag, None),
            candidate(Source::JatabagTz, "Asia/Tokyo"),
            candidate(Source::Tz, "Europe/Berlin"),
        ])
        .ok()
        .unwrap();

        assert_eq!("Asia/Tokyo", picked.name());
        assert_eq!(Source::JatabagTz, picked.source());
    }

    #[test]
    fn steps_over_a_source_holding_only_whitespace() {
        let picked = pick([
            candidate(Source::Tz, "   "),
            candidate(Source::OperatingSystem, "Europe/Berlin"),
        ])
        .ok()
        .unwrap();

        assert_eq!(Source::OperatingSystem, picked.source());
    }

    #[test]
    fn strips_the_colon_posix_allows_in_front_of_a_name() {
        let picked = pick([candidate(Source::Tz, ":America/Chicago")])
            .ok()
            .unwrap();

        assert_eq!("America/Chicago", picked.name());
    }

    #[test]
    fn accepts_a_posix_rule_from_tz() {
        let picked = pick([candidate(Source::Tz, "EST5EDT,M3.2.0,M11.1.0")])
            .ok()
            .unwrap();

        assert_eq!("EST5EDT,M3.2.0,M11.1.0", picked.name());
    }

    #[test]
    fn rejects_a_posix_rule_from_any_source_but_tz() {
        let error = pick([candidate(Source::Flag, "EST5EDT,M3.2.0,M11.1.0")])
            .err()
            .unwrap();

        assert!(matches!(error, Error::Unrecognized { .. }));
    }

    #[test]
    fn fails_rather_than_falling_through_to_a_later_source() {
        let error = pick([
            candidate(Source::Tz, "Mars/Olympus"),
            candidate(Source::OperatingSystem, "Europe/Berlin"),
        ])
        .err()
        .unwrap();

        assert!(matches!(
            error,
            Error::Unrecognized {
                source: Source::Tz,
                ..
            }
        ));
    }

    #[test]
    fn reports_no_timezone_when_every_source_is_empty() {
        let error = pick([(Source::Flag, None), (Source::Tz, None)])
            .err()
            .unwrap();

        assert!(matches!(error, Error::Undetermined));
    }
}
