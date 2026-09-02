use crate::target::{Dependencies, Dependency};
use color_eyre::eyre::Result;
use std::{collections::HashSet, path::Path};
use vizual::Signal;

pub fn normalize_path(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    if let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) {
        if let Ok(relative) = canonical.strip_prefix(&home) {
            if relative.as_os_str().is_empty() {
                return "~".to_string();
            }
            return format!("~/{}", relative.display());
        }
    }

    let path_str = canonical.display().to_string();
    if let Some(rest) = path_str.strip_prefix("/home/") {
        if let Some((_user, after_user)) = rest.split_once('/') {
            return format!("~/{after_user}");
        } else {
            return "~".to_string();
        }
    }

    path_str
}

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
