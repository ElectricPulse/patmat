use async_trait::async_trait;
use color_eyre::Result;
use derive_new::new;
use std::path::PathBuf;
use vizual::{
    component::{Children, context::Component_context},
    geometry::Direction,
    handlers::Retrieve_handler,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    widget::{
        Focus_provider, Layout_input, Widget, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{
            icon::Icon,
            layout::axis::Axis,
            menu::{Menu, Menu_item},
            positioning::anchor::{Anchor, Anchors, Position},
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
    async fn on_retrieve(&mut self) -> Result<Dependency> {
        Ok(self.target.clone())
    }
}

#[async_trait::async_trait]
impl Custom_widget_trait for Target_tree_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        Layout_input {
            render,
            slots,
            ..
        }: Layout_input<'_>,
        _selected: bool,
    ) -> Result<Children> {
        let metadata = self.target.get_metadata();

        let icon = Icon::new(metadata.status.affect(render.clone()).await?.get_icon());
        let icon = Anchor::new(
            icon,
            Anchors {
                vertical: Some(Position::Middle),
                horizontal: None,
            },
        );

        let name = Text::new(metadata.name);
        let name = Anchor::left(name);

        let mut details: Vec<Widget> = vec![Box::new(name)];

        let path = metadata.path.affect(render).await?.clone();
        if let Some(path) = path {
            let path = display_target_path(&path, &self.working_directory);
            let path = Text::new(format!("- {path}"));
            let path = Anchor::left(path);
            details.push(Box::new(path));
        }

        let details = Axis::new(Direction::Vertical, details);
        let details = Anchor::left(details);

        let row = Axis::new(
            Direction::Horizontal,
            vec![Box::new(details), Box::new(icon)],
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
        menu.submitted = self.selected.clone();

        Ok(vec![display!(menu)])
    }
}
