use super::*;

#[test]
fn clones_and_checks_out_a_local_repository() -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let destination = temporary.path().join("clone");
    let progress = gix::progress::tree::Root::new();

    let repo_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")));

    clone_repository(
        repo_dir.to_string_lossy().to_string(),
        destination.clone(),
        progress,
    )?;

    assert!(destination.join(".git").is_dir());
    assert!(destination.join("Cargo.toml").is_file());
    Ok(())
}
