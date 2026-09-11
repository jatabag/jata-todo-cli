use std::env;
use std::fmt;
use uuid::Uuid;

const DEFAULT_BASE: &str = "http://localhost";

pub enum Error {
    Unreachable {
        url: String,
        cause: Box<ureq::Error>,
    },
    Missing {
        tree: Uuid,
        base: String,
    },
    Status {
        code: u16,
        tree: Uuid,
    },
    Unreadable(Box<ureq::Error>),
    Refused {
        code: u16,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unreachable { url, cause } => write!(f, "{url}: {cause}"),
            Error::Missing { tree, base } => write!(
                f,
                "{base} has no tree {tree} — it may have been deleted, or the trees \
                 may live somewhere else (set JATABAG_URL or pass --url)"
            ),
            Error::Status { code, tree } => write!(f, "{tree} answered {code}"),
            Error::Unreadable(cause) => write!(f, "{cause}"),
            Error::Refused { code } => write!(f, "the tree refused the task, answering {code}"),
        }
    }
}

/// Where the tasks live: `--url`, then `JATABAG_URL`, then a local instance.
pub fn base(flag: Option<&str>) -> String {
    let chosen = flag
        .map(str::to_owned)
        .or_else(|| env::var("JATABAG_URL").ok())
        .filter(|url| !url.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_BASE.to_owned());

    chosen.trim().trim_end_matches('/').to_owned()
}

/// The plain-text render of a tree.
///
/// The `.txt` extension asks for it by name rather than by header, so no
/// negotiation stands between a tree and a program reading it.
pub fn plaintext(base: &str, tree: Uuid, query: &[(String, String)]) -> Result<String, Error> {
    let url = format!("{base}/tree/{}.txt", tree.hyphenated());

    let call = ureq::get(&url)
        .query_pairs(
            query
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        )
        .call();

    match call {
        Ok(response) => response
            .into_body()
            .read_to_string()
            .map_err(|cause| Error::Unreadable(Box::new(cause))),
        Err(ureq::Error::StatusCode(404)) => Err(Error::Missing {
            tree,
            base: base.to_owned(),
        }),
        Err(ureq::Error::StatusCode(code)) => Err(Error::Status { code, tree }),
        Err(cause) => Err(Error::Unreachable {
            url,
            cause: Box::new(cause),
        }),
    }
}

/// Add a task to a tree.
///
/// The timezone rides along as the cookie the web app sends, because the server
/// dates what it records from it and falls back to UTC without one.
pub fn create_task(
    base: &str,
    tree: Uuid,
    timezone: &str,
    fields: &[(String, String)],
) -> Result<(), Error> {
    let url = format!("{base}/tree/{}/task", tree.hyphenated());

    let call = ureq::post(&url)
        .config()
        .max_redirects(0)
        .build()
        .header("Cookie", format!("tz={timezone}"))
        .send_form(
            fields
                .iter()
                .map(|(key, value)| (key.as_str(), value.as_str())),
        );

    match call {
        Ok(response) => match response.status().as_u16() {
            200..=303 => Ok(()),
            code => Err(Error::Refused { code }),
        },
        Err(ureq::Error::StatusCode(404)) => Err(Error::Missing {
            tree,
            base: base.to_owned(),
        }),
        Err(ureq::Error::StatusCode(code)) => Err(Error::Status { code, tree }),
        Err(cause) => Err(Error::Unreachable {
            url,
            cause: Box::new(cause),
        }),
    }
}

/// Remove a task, and with `purge` the subtree standing under it rather than
/// letting those subtasks rise to its parent.
pub fn delete_task(
    base: &str,
    tree: Uuid,
    task: Uuid,
    purge: bool,
    timezone: &str,
) -> Result<(), Error> {
    let url = format!(
        "{base}/tree/{}/task/{}",
        tree.hyphenated(),
        task.hyphenated()
    );

    let body = if purge { "c=1" } else { "" };

    let request = ureq::http::Request::builder()
        .method("DELETE")
        .uri(&url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("Cookie", format!("tz={timezone}"))
        .body(body);

    let Ok(request) = request else {
        return Err(Error::Status { code: 0, tree });
    };

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .max_redirects(0)
        .build()
        .into();

    match agent.run(request) {
        Ok(response) => match response.status().as_u16() {
            200..=303 => Ok(()),
            code => Err(Error::Refused { code }),
        },
        Err(ureq::Error::StatusCode(404)) => Err(Error::Missing {
            tree,
            base: base.to_owned(),
        }),
        Err(ureq::Error::StatusCode(code)) => Err(Error::Status { code, tree }),
        Err(cause) => Err(Error::Unreachable {
            url,
            cause: Box::new(cause),
        }),
    }
}
