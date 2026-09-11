use uuid::Uuid;

const HYPHENATED_LEN: usize = 36;

/// Every tree id held anywhere in `text`, in the order they appear, each one
/// canonicalized and listed once.
///
/// Ids are read wherever they sit rather than only where a url would put them,
/// because what gets pasted is whatever was to hand: an address bar, a share
/// sheet, a line of a chat log. A tree names itself the same in all of them.
pub fn extract(text: &str) -> Vec<Uuid> {
    let bytes = text.as_bytes();
    let mut found: Vec<Uuid> = Vec::new();
    let mut at = 0;

    while at + HYPHENATED_LEN <= bytes.len() {
        let window = &bytes[at..at + HYPHENATED_LEN];

        let Ok(id) = Uuid::try_parse_ascii(window) else {
            at += 1;
            continue;
        };

        if !found.contains(&id) {
            found.push(id);
        }

        at += HYPHENATED_LEN;
    }

    found
}

#[cfg(test)]
mod extract {
    use super::*;

    const ONE: &str = "2e4e6863-9bfe-4c37-b64f-aa6da5ba51eb";
    const TWO: &str = "0193a1b2-c3d4-7e5f-8a9b-0c1d2e3f4a5b";

    fn extracted(text: &str) -> Vec<String> {
        extract(text)
            .iter()
            .map(|id| id.hyphenated().to_string())
            .collect()
    }

    #[test]
    fn reads_an_id_standing_alone() {
        assert_eq!(vec![ONE], extracted(ONE));
    }

    #[test]
    fn reads_an_id_with_text_pressed_against_both_ends() {
        assert_eq!(vec![ONE], extracted(&format!("lskdflskdjf{ONE}lkdjf")));
    }

    #[test]
    fn reads_an_id_out_of_a_link_carrying_a_query() {
        assert_eq!(
            vec![ONE],
            extracted(&format!("https://jatabag.test/tree/{ONE}?sort=stale-most"))
        );
    }

    #[test]
    fn reads_the_id_of_a_trees_other_renders() {
        assert_eq!(vec![ONE], extracted(&format!("/tree/{ONE}.txt")));
        assert_eq!(vec![ONE], extracted(&format!("/tree/{ONE}/graph")));
    }

    #[test]
    fn reads_every_id_in_the_order_they_appear() {
        assert_eq!(vec![ONE, TWO], extracted(&format!("a{ONE} b{TWO}")));
    }

    #[test]
    fn lists_an_id_once_however_many_times_it_appears() {
        assert_eq!(vec![ONE], extracted(&format!("{ONE} {ONE}")));
    }

    #[test]
    fn holds_ids_in_one_case_so_the_same_tree_is_one_entry() {
        assert_eq!(vec![ONE], extracted(&ONE.to_uppercase()));
        assert_eq!(
            vec![ONE],
            extracted(&format!("{ONE} {}", ONE.to_uppercase()))
        );
    }

    #[test]
    fn finds_nothing_in_text_holding_no_id() {
        assert!(extracted("just some words").is_empty());
        assert!(extracted("").is_empty());
    }

    #[test]
    fn passes_over_a_run_of_hex_that_is_not_shaped_like_an_id() {
        assert!(extracted("2e4e68639bfe4c37b64faa6da5ba51eb").is_empty());
        assert!(extracted("2e4e6863-9bfe-4c37-b64f-aa6da5ba51e").is_empty());
    }

    #[test]
    fn passes_over_a_group_holding_something_other_than_hex() {
        assert!(extracted("2e4e6863-9bfe-4c37-b64f-aa6da5ba51eg").is_empty());
    }
}
