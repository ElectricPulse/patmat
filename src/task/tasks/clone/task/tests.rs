use super::*;

#[test]
fn clones_and_checks_out_a_local_repository() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let destination = temporary.path().join("clone");
    let progress = gix::progress::tree::Root::new();

    clone_repository(
        env!("CARGO_MANIFEST_DIR").to_string(),
        destination.clone(),
        progress,
    )?;

    assert!(destination.join(".git").is_dir());
    assert!(destination.join("Cargo.toml").is_file());
    Ok(())
}
