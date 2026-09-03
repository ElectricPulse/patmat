use super::*;

#[test]
fn snapshots_bounded_gix_progress() {
    let tree = gix::progress::tree::Root::new();
    let item = tree.add_child("receiving objects");
    item.init(Some(20), gix::progress::count("objects"));
    item.set(5);

    let CloneProgress::Running(rows) = CloneProgress::from_tree(&tree) else {
        panic!("a live gix task should produce running progress");
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "receiving objects");
    assert_eq!(rows[0].fraction, Some(0.25));
}

#[test]
fn remote_progress_names_are_kept_to_one_bounded_line() {
    let name = format!("{}\nignored", "x".repeat(MAX_NAME_CHARACTERS + 10));
    let name = single_line(&name);

    assert_eq!(name.chars().count(), MAX_NAME_CHARACTERS + 1);
    assert!(name.ends_with('…'));
    assert!(!name.contains('\n'));
}
