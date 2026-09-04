use crate::target::{Dependencies, Dependency};
use color_eyre::eyre::Result;
use std::{collections::HashSet, path::Path};
use vizual::Signal;


pub(crate) fn display_target_path(path: &Path, working_directory: &Path) -> String {
    let path = path.strip_prefix(working_directory).unwrap_or(path);
    match path.as_os_str().is_empty() {
        true => ".".to_owned(),
        false => path.display().to_string(),
    }
}

pub(crate) async fn get_targets(roots: &[Dependency], relayout: Signal) -> Result<Dependencies> {
    let mut remaining = roots.iter().rev().cloned().collect::<Dependencies>();
    let mut unique = Dependencies::new();
    let mut target_ids = HashSet::new();

    while let Some(target) = remaining.pop() {
        let metadata = target.get_metadata();
        let id = *metadata.id.affect(relayout.clone()).await?;
        let dependencies = metadata
            .dependencies
            .affect(relayout.clone())
            .await?
            .clone();
        if target_ids.insert(id) {
            remaining.extend(dependencies.into_iter().rev());
            unique.push(target);
        }
    }

    Ok(unique)
}
