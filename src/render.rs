use clap::{Args, ValueEnum};

/// The lens settings a plain-text render honours, named here as they are named
/// in the query the server reads.
#[derive(Args)]
pub struct Options {
    /// Append each task's id, which is what addresses it in every other command
    #[arg(long)]
    pub ids: bool,

    /// Append each task's inline #tags
    #[arg(long)]
    pub tags: bool,

    /// Leave out the note lines beneath tasks
    #[arg(long)]
    pub no_context: bool,

    /// Write linked tasks as [text](url)
    #[arg(long)]
    pub links: bool,

    /// Keep only tasks carrying this tag; repeat for several
    #[arg(long = "tag", value_name = "TAG")]
    pub filter_tags: Vec<String>,

    /// Whether several --tag values must all match, or any of them
    #[arg(long, value_name = "MODE", default_value = "or")]
    pub tag_mode: TagMode,

    /// Keep the tasks the --tag values do not match
    #[arg(long)]
    pub without: bool,

    /// Read from this task down, making it the top of the tree
    #[arg(long, value_name = "TASK_ID")]
    pub under: Option<String>,

    /// Treat tasks above this percent as over-pressure
    #[arg(long, value_name = "PERCENT")]
    pub pressure: Option<u8>,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TagMode {
    Or,
    And,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Sort {
    Default,
    CreatedNewest,
    CreatedOldest,
    StaleMost,
    StaleLeast,
    WeightLightest,
    WeightHeaviest,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum SortScope {
    Tasks,
    Categories,
    Both,
}

fn spelling(value: impl ValueEnum) -> String {
    value
        .to_possible_value()
        .map(|possible| possible.get_name().to_owned())
        .unwrap_or_default()
}

impl Options {
    /// Only the settings that were asked for, so the server's own defaults
    /// answer for everything else.
    pub fn query(&self) -> Vec<(String, String)> {
        let mut query = Vec::new();

        let mut switch = |name: &str, on: bool, value: &str| {
            if on {
                query.push((format!("plaintext[{name}]"), value.to_owned()));
            }
        };

        switch("ids", self.ids, "on");
        switch("tags", self.tags, "on");
        switch("links", self.links, "on");
        switch("context", self.no_context, "off");

        for tag in &self.filter_tags {
            query.push(("filter[tags][]".to_owned(), tag.clone()));
        }

        if !self.filter_tags.is_empty() && self.tag_mode == TagMode::And {
            query.push(("filter[mode]".to_owned(), "and".to_owned()));
        }

        if self.without {
            query.push(("filter[not]".to_owned(), "1".to_owned()));
        }

        if let Some(task) = &self.under {
            query.push(("t_nid".to_owned(), task.clone()));
        }

        if let Some(pressure) = self.pressure {
            query.push(("pressure".to_owned(), pressure.to_string()));
        }

        query
    }
}

#[derive(Args)]
pub struct FocusOptions {
    /// Collapse each category to a single breadcrumb heading
    #[arg(long)]
    pub flat: bool,

    /// How tasks are ordered within each group
    #[arg(long, value_name = "ORDER")]
    pub sort: Option<Sort>,

    /// How far the sort reaches
    #[arg(long, value_name = "SCOPE")]
    pub sort_scope: Option<SortScope>,
}

impl FocusOptions {
    pub fn query(&self) -> Vec<(String, String)> {
        let mut query = vec![("view".to_owned(), "focus".to_owned())];

        if self.flat {
            query.push(("plaintext[flat]".to_owned(), "on".to_owned()));
        }

        if let Some(sort) = self.sort {
            query.push(("sort".to_owned(), spelling(sort)));
        }

        if let Some(scope) = self.sort_scope {
            query.push(("sort_scope".to_owned(), spelling(scope)));
        }

        query
    }
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Position {
    Top,
    Bottom,
}

impl Position {
    pub fn spelling(self) -> String {
        spelling(self)
    }
}
