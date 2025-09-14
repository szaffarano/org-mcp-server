use nucleo_matcher::{
    Config, Matcher,
    pattern::{AtomKind, CaseMatching, Normalization, Pattern},
};

fn main() {
    let mut matcher = Matcher::new(Config::DEFAULT);
    let content = r#"
    This is a text
    to check nucleo
    and also add more
    texts
    "#;

    let lines = content.lines().map(|s| s.to_owned()).collect::<Vec<_>>();

    let matches = Pattern::new(
        "text",
        CaseMatching::Ignore,
        Normalization::Smart,
        AtomKind::Fuzzy,
    )
    .match_list(lines, &mut matcher);

    println!(">>> {:?}", matches);
}
