use crate::model::ChecklistItem;

const TEMPLATE: [&str; 5] = [
    "Store page updated",
    "Trailer uploaded",
    "Achievements tested",
    "Steam Deck tested",
    "Patch notes written",
];

pub fn template() -> Vec<ChecklistItem> {
    TEMPLATE
        .iter()
        .map(|t| ChecklistItem {
            text: (*t).to_string(),
            done: false,
        })
        .collect()
}

pub fn reset(items: &mut [ChecklistItem]) {
    for item in items.iter_mut() {
        item.done = false;
    }
}

pub fn unchecked(items: &[ChecklistItem]) -> Vec<&str> {
    items
        .iter()
        .filter(|i| !i.done && !i.text.trim().is_empty())
        .map(|i| i.text.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(text: &str, done: bool) -> ChecklistItem {
        ChecklistItem {
            text: text.to_string(),
            done,
        }
    }

    #[test]
    fn template_items_start_unchecked() {
        let t = template();
        assert_eq!(t.len(), 5);
        assert!(t.iter().all(|i| !i.done));
        assert_eq!(t[0].text, "Store page updated");
    }

    #[test]
    fn unchecked_skips_done_and_blank() {
        let items = vec![
            item("a", false),
            item("b", true),
            item("   ", false),
            item("c", false),
        ];
        assert_eq!(unchecked(&items), vec!["a", "c"]);
    }

    #[test]
    fn reset_clears_done() {
        let mut items = vec![item("a", true), item("b", true)];
        reset(&mut items);
        assert!(items.iter().all(|i| !i.done));
    }
}
