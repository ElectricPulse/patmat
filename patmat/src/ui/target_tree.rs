use async_trait::async_trait;
use color_eyre::Result;
use derive_new::new;
use lucide_icons::Icon as Lucide_icon;
use std::path::PathBuf;
use vizual::{
    component::Children,
    geometry::Direction,
    handlers::Retrieve_handler,
    state::{State, Store},
    widget::{
        Layout_input, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{
            icon::Icon,
            layout::axis::Axis,
            menu::{Menu, Menu_item},
            positioning::anchor::{Anchor, Anchors, Anchor_position},
            text::Text,
        },
    },
};
use vizual_macros::display;

use crate::target::{Dependencies, Dependency};
use crate::utils::{display_target_path, get_targets};

#[derive(Clone, new)]
struct Target_tree_item {
    target: Dependency,
    working_directory: PathBuf,
}

#[async_trait::async_trait]
impl Retrieve_handler<Dependency> for Target_tree_item {
    async fn on_retrieve(&mut self) -> Result<State<Dependency>> {
        Ok(self.target.clone().into())
    }
}

#[async_trait::async_trait]
impl Custom_widget_trait for Target_tree_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
        selected: bool,
    ) -> Result<Children> {
        let theme = theme.affect(render.clone()).await?;
        let metadata = self.target.get_metadata();

        let icon = Icon::new(metadata.status.affect(render.clone()).await?.get_icon());
        let icon = Anchor::new(
            icon,
            Anchors {
                vertical: Some(Anchor_position::Middle),
                horizontal: None,
            },
        );

        let mut name = Text::new(metadata.name);
        let mut name_style = theme.specific.text.paragraph;
        if !selected {
            name_style.color = theme.semantic.text.muted;
        }
        name.style.set(name_style);
        let name = Anchor::left(name);

        let path = metadata.path.affect(render).await?.clone();
        let details = match path {
            Some(path) => {
                let path = display_target_path(&path, &self.working_directory);
                let mut path_text = Text::new(path);
                let mut path_style = theme.specific.text.paragraph;
                path_style.color = theme.semantic.text.muted;
                path_text.style.set(path_style);
                let mut folder_icon = Icon::new(Lucide_icon::Folder);
                folder_icon.style.set(path_style);
                let path_row = Axis::new(
                    Direction::Horizontal,
                    (Anchor::left(folder_icon), Anchor::left(path_text)),
                );
                let path = Anchor::left(path_row);
                Axis::new(Direction::Vertical, (name, path))
            }
            None => Axis::new(Direction::Vertical, (name,)),
        };
        let details = Anchor::left(details);

        let row = Axis::new(
            Direction::Horizontal,
            (details, icon),
        );

        Ok(vec![display!(row)])
    }
}

#[derive(Clone)]
pub struct Target_tree {
    dependencies: Dependencies,
    selected: Store<Dependency>,
    selected_index: Store<usize>,
    working_directory: PathBuf,
}

impl Target_tree {
    pub fn new(
        dependencies: Dependencies,
        selected: Store<Dependency>,
        working_directory: PathBuf,
    ) -> Self {
        Self {
            dependencies,
            selected,
            selected_index: Store::new(0),
            working_directory,
        }
    }
}

#[async_trait]
impl Widget_trait for Target_tree {
    async fn layout(
        &mut self,
        Layout_input {
            render,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let targets = get_targets(&self.dependencies, render.clone())
            .await?
            .into_iter()
            .map(|target| -> Menu_item<Dependency> {
                Box::new(Target_tree_item::new(
                    target,
                    self.working_directory.clone(),
                ))
            })
            .collect::<Vec<_>>();

        if targets.is_empty() {
            return Ok(vec![]);
        }

        let selected_index = *self.selected_index.read().await?;
        let default_index = selected_index.min(targets.len().saturating_sub(1));
        let mut menu = Menu::new(targets, default_index).await?;
        menu.selected = self.selected_index.clone();
        menu.set_submitted(self.selected.clone()).await?;

        Ok(vec![display!(menu)])
    }
}
