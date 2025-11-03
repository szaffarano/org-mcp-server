use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;

fn main() {
    let home = shellexpand::tilde("~/Documents/**/*.org").into_owned();
    let root = home
        .split_once('*')
        .map(|(prefix, _)| prefix)
        .unwrap_or(&home);

    let globset = GlobSetBuilder::new()
        .add(Glob::new(&home).expect("invalid glob"))
        .build()
        .unwrap();

    for entry in WalkBuilder::new(root).build().flatten() {
        let path = entry.path();
        if path.is_file() && globset.is_match(path) {
            println!("{}", path.display());
        }
    }
}
