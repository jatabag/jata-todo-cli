mod api;
mod pick;
mod render;
mod store;
mod task;
mod timezone;
mod tree_id;

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{CommandFactory, Parser, Subcommand};
use std::process::ExitCode;
use store::Store;

/// Terminal colours rather than fixed ones, so the palette a terminal is
/// themed with is the palette this help is drawn in.
const STYLES: Styles = Styles::styled()
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .header(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Magenta.on_default())
    .valid(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .invalid(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
    .error(AnsiColor::Red.on_default().effects(Effects::BOLD));

#[derive(Parser)]
#[command(
    version,
    about,
    long_about = None,
    allow_external_subcommands = true,
    styles = STYLES
)]
struct Cli {
    /// The timezone to work in, overriding JATABAG_TZ, TZ and the operating system
    #[arg(long, short = 'z', global = true, value_name = "TIMEZONE")]
    timezone: Option<String>,

    /// Where the trees live, overriding JATABAG_URL
    #[arg(long, global = true, value_name = "URL")]
    url: Option<String>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Print a tree as plaintext
    Tree {
        #[command(flatten)]
        options: render::Options,
    },
    /// Print a tree's Focus view — what is actionable — as plaintext
    Focus {
        #[command(flatten)]
        options: render::Options,

        #[command(flatten)]
        focus: render::FocusOptions,
    },
    /// Create a task on the tree
    #[command(visible_aliases = ["create", "add"])]
    New {
        /// The task's text
        #[arg(value_name = "TEXT", required = true, num_args = 1..)]
        text: Vec<String>,

        /// Notes to keep beneath it
        #[arg(long, short = 'c', visible_alias = "context", value_name = "NOTES")]
        notes: Option<String>,

        /// Put it beneath this task rather than at the top level
        #[arg(long, value_name = "TASK_ID")]
        under: Option<String>,

        /// Make it a category rather than a task
        #[arg(long)]
        category: bool,

        /// Where among the parent's tasks it lands
        #[arg(long, value_name = "WHERE")]
        position: Option<render::Position>,
    },
    /// Delete a task, chosen from the tree's own tasks
    #[command(visible_alias = "remove")]
    Delete {
        /// Take the task's whole subtree with it, rather than letting its
        /// subtasks rise to its parent
        #[arg(long)]
        purge: bool,
    },
    /// Print the timezone being worked in
    Tz {
        /// Name where the timezone was found alongside it
        #[arg(long)]
        source: bool,
    },
    /// Print the trees kept on this machine
    List,
    /// Print where the kept tree ids are stored
    Where,
    /// Anything holding tree ids — a link, several links, a line you pasted
    #[command(external_subcommand)]
    Keep(Vec<String>),
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let outcome = match cli.command {
        Some(Command::Tree { options }) => print_plaintext(cli.url.as_deref(), options.query()),
        Some(Command::Focus { options, focus }) => {
            let mut query = options.query();
            query.extend(focus.query());

            print_plaintext(cli.url.as_deref(), query)
        }
        Some(Command::New {
            text,
            notes,
            under,
            category,
            position,
        }) => create_task(
            cli.url.as_deref(),
            cli.timezone.as_deref(),
            &text.join(" "),
            notes,
            under,
            category,
            position,
        ),
        Some(Command::Delete { purge }) => {
            delete_task(cli.url.as_deref(), cli.timezone.as_deref(), purge)
        }
        Some(Command::Tz { source }) => print_timezone(cli.timezone.as_deref(), source),
        Some(Command::List) => list_trees(),
        Some(Command::Where) => print_path(),
        Some(Command::Keep(text)) => keep_trees(&text.join(" ")),
        None => print_help(),
    };

    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(report) => {
            eprintln!("{report}");
            ExitCode::FAILURE
        }
    }
}

fn print_help() -> Result<(), String> {
    Cli::command()
        .print_help()
        .map_err(|error| error.to_string())
}

fn print_timezone(flag: Option<&str>, source: bool) -> Result<(), String> {
    let timezone = timezone::resolve(flag).map_err(|error| error.to_string())?;

    if source {
        println!("{} (from {})", timezone.name(), timezone.source());
    } else {
        println!("{}", timezone.name());
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_task(
    url: Option<&str>,
    timezone: Option<&str>,
    text: &str,
    notes: Option<String>,
    under: Option<String>,
    category: bool,
    position: Option<render::Position>,
) -> Result<(), String> {
    let timezone = timezone::resolve(timezone).map_err(|error| error.to_string())?;
    let tree = chosen_tree()?;

    let mut fields = vec![
        ("mode".to_owned(), "one".to_owned()),
        ("l".to_owned(), text.to_owned()),
    ];

    if let Some(notes) = notes {
        fields.push(("c".to_owned(), notes));
    }

    if let Some(under) = under {
        fields.push(("n".to_owned(), under));
    }

    if category {
        fields.push(("category".to_owned(), "1".to_owned()));
    }

    if let Some(position) = position {
        fields.push(("p".to_owned(), position.spelling()));
    }

    api::create_task(&api::base(url), tree, timezone.name(), &fields)
        .map_err(|error| error.to_string())?;

    println!("added \"{text}\"");

    Ok(())
}

fn delete_task(url: Option<&str>, timezone: Option<&str>, purge: bool) -> Result<(), String> {
    let timezone = timezone::resolve(timezone).map_err(|error| error.to_string())?;
    let base = api::base(url);
    let tree = chosen_tree()?;

    let listing =
        api::plaintext(&base, tree, &naming_tasks()).map_err(|error| error.to_string())?;
    let task = pick::a_task(task::from_plaintext(&listing)).map_err(|error| error.to_string())?;

    api::delete_task(&base, tree, task.id, purge, timezone.name())
        .map_err(|error| error.to_string())?;

    if purge {
        println!("deleted \"{}\" and everything under it", task.text());
    } else {
        println!("deleted \"{}\"", task.text());
    }

    Ok(())
}

/// The render that names every task, which is the one a task can be picked out of.
fn naming_tasks() -> Vec<(String, String)> {
    vec![
        ("plaintext[ids]".to_owned(), "on".to_owned()),
        ("plaintext[context]".to_owned(), "off".to_owned()),
    ]
}

fn chosen_tree() -> Result<uuid::Uuid, String> {
    let store = Store::open().map_err(|error| error.to_string())?;
    let trees = store.read().map_err(|error| error.to_string())?;

    pick::one_of(trees).map_err(|error| error.to_string())
}

fn print_plaintext(url: Option<&str>, query: Vec<(String, String)>) -> Result<(), String> {
    let tree = chosen_tree()?;

    let body = api::plaintext(&api::base(url), tree, &query).map_err(|error| error.to_string())?;

    print!("{body}");

    Ok(())
}

fn print_path() -> Result<(), String> {
    let store = Store::open().map_err(|error| error.to_string())?;

    println!("{}", store.path().display());

    Ok(())
}

fn list_trees() -> Result<(), String> {
    let store = Store::open().map_err(|error| error.to_string())?;

    for id in store.read().map_err(|error| error.to_string())? {
        println!("{}", id.hyphenated());
    }

    Ok(())
}

fn keep_trees(text: &str) -> Result<(), String> {
    let found = tree_id::extract(text);

    if found.is_empty() {
        return Err(
            "no tree id in that — a tree id looks like 2e4e6863-9bfe-4c37-b64f-aa6da5ba51eb"
                .to_owned(),
        );
    }

    let store = Store::open().map_err(|error| error.to_string())?;
    let added = store.add(&found).map_err(|error| error.to_string())?;

    for id in &added {
        println!("{}", id.hyphenated());
    }

    let known = found.len() - added.len();

    if known > 0 {
        eprintln!("{known} already kept");
    }

    Ok(())
}
