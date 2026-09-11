use crate::task::Task;
use nucleo_picker::{Picker, error::PickError};
use std::borrow::Cow;
use std::fmt;
use uuid::Uuid;

pub enum Error {
    Nothing,
    NoTasks,
    Abandoned,
    NotInteractive(&'static str),
    Failed(PickError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Nothing => f.write_str("no trees kept — run jata with a link to keep one"),
            Error::NoTasks => f.write_str("that tree has no tasks"),
            Error::Abandoned => f.write_str("nothing chosen"),
            Error::NotInteractive(what) => {
                write!(f, "choosing a {what} needs a terminal to choose in")
            }
            Error::Failed(cause) => write!(f, "{cause}"),
        }
    }
}

fn tree_label(tree: &Uuid) -> Cow<'_, str> {
    Cow::Owned(tree.hyphenated().to_string())
}

fn task_label(task: &Task) -> Cow<'_, str> {
    Cow::Borrowed(task.line())
}

/// One of them, chosen the way fzf chooses: type to narrow, enter to take.
fn among<T>(
    items: Vec<T>,
    label: for<'a> fn(&'a T) -> Cow<'a, str>,
    what: &'static str,
) -> Result<T, Error>
where
    T: Clone + Send + Sync + 'static,
{
    let mut picker = Picker::new(label);

    picker.extend(items);

    match picker.pick() {
        Ok(Some(chosen)) => Ok(chosen.clone()),
        Ok(None) | Err(PickError::UserInterrupted) => Err(Error::Abandoned),
        Err(PickError::NotInteractive) => Err(Error::NotInteractive(what)),
        Err(cause) => Err(Error::Failed(cause)),
    }
}

/// The task to act on, chosen out of the tree's own tasks.
pub fn a_task(tasks: Vec<Task>) -> Result<Task, Error> {
    if tasks.is_empty() {
        return Err(Error::NoTasks);
    }

    among(tasks, task_label, "task")
}

/// The tree to work on: the only one kept, or the one picked out of them.
pub fn one_of(trees: Vec<Uuid>) -> Result<Uuid, Error> {
    match trees.len() {
        0 => Err(Error::Nothing),
        1 => Ok(trees[0]),
        _ => choose(trees),
    }
}

fn choose(trees: Vec<Uuid>) -> Result<Uuid, Error> {
    among(trees, tree_label, "tree")
}
