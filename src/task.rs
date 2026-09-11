use uuid::Uuid;
use uuid::fmt::Hyphenated;

const CARET: char = '^';
const MARKER: usize = CARET.len_utf8() + Hyphenated::LENGTH;

#[derive(Clone)]
pub struct Task {
    pub id: Uuid,
    line: String,
}

impl Task {
    pub fn line(&self) -> &str {
        &self.line
    }

    pub fn text(&self) -> &str {
        let bare = self.line.trim_start();

        if let Some(task) = bare.strip_prefix("- ") {
            return task;
        }

        let heading = bare.trim_start_matches('#');

        if heading.len() < bare.len() {
            return heading.trim_start();
        }

        bare
    }
}

pub fn from_plaintext(body: &str) -> Vec<Task> {
    body.lines().filter_map(marked).collect()
}

fn marked(line: &str) -> Option<Task> {
    let line = line.trim_end();
    let caret = line.len().checked_sub(MARKER)?;

    if !line.is_char_boundary(caret) {
        return None;
    }

    let (text, marker) = line.split_at(caret);
    let id = Uuid::try_parse(marker.strip_prefix(CARET)?).ok()?;

    Some(Task {
        id,
        line: text.trim_end().to_owned(),
    })
}

#[cfg(test)]
mod from_plaintext {
    use super::*;

    const ID: &str = "01a0653f-a1fd-72f7-bda8-f68b632d520f";

    fn lines(body: &str) -> Vec<String> {
        from_plaintext(body)
            .iter()
            .map(|task| task.line().to_owned())
            .collect()
    }

    #[test]
    fn reads_a_task_and_keeps_the_line_it_was_drawn_on() {
        let tasks = from_plaintext(&format!("- Buy milk ^{ID}"));

        assert_eq!(1, tasks.len());
        assert_eq!(ID, tasks[0].id.hyphenated().to_string());
        assert_eq!("- Buy milk", tasks[0].line());
    }

    #[test]
    fn keeps_the_indent_a_subtask_is_drawn_with() {
        assert_eq!(
            vec!["  - Design the listing"],
            lines(&format!("  - Design the listing ^{ID}"))
        );
    }

    #[test]
    fn reads_a_category_as_readily_as_a_task() {
        assert_eq!(vec!["# Errands"], lines(&format!("# Errands ^{ID}")));
    }

    #[test]
    fn passes_over_the_lines_that_carry_no_marker() {
        let body = format!("- Buy milk ^{ID}\n\n  a note about milk\n\n---\n");

        assert_eq!(vec!["- Buy milk"], lines(&body));
    }

    #[test]
    fn passes_over_a_caret_that_is_not_followed_by_an_id() {
        assert!(lines("- Read Knuth vol ^2").is_empty());
        assert!(lines("- Caret at the end ^").is_empty());
        assert!(lines(&format!("- Not an id ^{}", "z".repeat(36))).is_empty());
    }

    #[test]
    fn passes_over_an_id_that_is_not_behind_a_caret() {
        assert!(lines(&format!("- Buy milk {ID}")).is_empty());
    }

    #[test]
    fn passes_over_an_id_that_is_not_where_the_marker_goes() {
        assert!(lines(&format!("- Talk about ^{ID} with them")).is_empty());
    }

    #[test]
    fn takes_the_marker_at_the_end_and_leaves_a_caret_in_the_text() {
        let tasks = from_plaintext(&format!("- Mark it ^here ^{ID}"));

        assert_eq!(
            vec!["- Mark it ^here"],
            lines(&format!("- Mark it ^here ^{ID}"))
        );
        assert_eq!("Mark it ^here", tasks[0].text());
    }
}

#[cfg(test)]
mod text {
    use super::*;

    const ID: &str = "01a0653f-a1fd-72f7-bda8-f68b632d520f";

    fn text(line: &str) -> String {
        from_plaintext(&format!("{line} ^{ID}"))[0]
            .text()
            .to_owned()
    }

    #[test]
    fn drops_the_indent_and_the_task_marker() {
        assert_eq!("Design the listing", text("    - Design the listing"));
    }

    #[test]
    fn drops_a_category_marker_however_deep() {
        assert_eq!("Errands", text("# Errands"));
        assert_eq!("Errands", text("### Errands"));
    }

    #[test]
    fn keeps_a_dash_that_belongs_to_the_task() {
        assert_eq!("- dash first", text("- - dash first"));
    }
}
