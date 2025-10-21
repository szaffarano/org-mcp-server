/// Checks if all tags in `filter_tags` are present in `item_tags`.
pub fn tags_match(item_tags: &[String], filter_tags: &[String]) -> bool {
    if item_tags.is_empty() && !filter_tags.is_empty() {
        return false;
    }
    for tag in filter_tags {
        if !item_tags.contains(tag) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tags_match_both_empty() {
        let item_tags: Vec<String> = vec![];
        let filter_tags: Vec<String> = vec![];
        assert!(tags_match(&item_tags, &filter_tags));
    }

    #[test]
    fn test_tags_match_empty_items_with_filter() {
        let item_tags: Vec<String> = vec![];
        let filter_tags = vec!["tag1".to_string()];
        assert!(!tags_match(&item_tags, &filter_tags));
    }

    #[test]
    fn test_tags_match_all_present() {
        let item_tags = vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()];
        let filter_tags = vec!["tag1".to_string(), "tag2".to_string()];
        assert!(tags_match(&item_tags, &filter_tags));
    }

    #[test]
    fn test_tags_match_partial_match() {
        let item_tags = vec!["tag1".to_string(), "tag2".to_string()];
        let filter_tags = vec!["tag1".to_string(), "tag3".to_string()];
        assert!(!tags_match(&item_tags, &filter_tags));
    }

    #[test]
    fn test_tags_match_subset() {
        let item_tags = vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()];
        let filter_tags = vec!["tag2".to_string()];
        assert!(tags_match(&item_tags, &filter_tags));
    }

    #[test]
    fn test_tags_match_empty_filter() {
        let item_tags = vec!["tag1".to_string(), "tag2".to_string()];
        let filter_tags: Vec<String> = vec![];
        assert!(tags_match(&item_tags, &filter_tags));
    }

    #[test]
    fn test_tags_match_exact() {
        let item_tags = vec!["tag1".to_string(), "tag2".to_string()];
        let filter_tags = vec!["tag1".to_string(), "tag2".to_string()];
        assert!(tags_match(&item_tags, &filter_tags));
    }
}
