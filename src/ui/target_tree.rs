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
        Focus_provider, Shared_widget, Widget, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{
            icon::Icon,
            layout::axis::Axis,
            menu::{Menu, Shared_menu_item, get_selector},
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
impl Retrieve_handler<Option<Dependency>> for Target_tree_item {
    async fn on_retrieve(&mut self) -> Result<Option<Dependency>> {
        Ok(Some(self.target.clone()))
    }
}

#[async_trait::async_trait]
impl Custom_widget_trait for Target_tree_item {
    type Payload = bool;

    async fn layout(
        &mut self,
        render: vizual::Render,
        _theme: Store<vizual::theme::Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::graphics::text::Text_context,
        slots: &mut Slots,
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
    selected: Store<Option<Dependency>>,
    working_directory: PathBuf,
    menu: Option<Shared_widget<Menu<Option<Dependency>>>>,
}

impl Target_tree {
    pub fn new(
        dependencies: Dependencies,
        selected: Store<Option<Dependency>>,
        working_directory: PathBuf,
    ) -> Self {
        Self {
            dependencies,
            selected,
            working_directory,
            menu: None,
        }
    }
}

#[async_trait]
impl Widget_trait for Target_tree {
    async fn layout(
        &mut self,
        render: vizual::Render,
        _theme: Store<vizual::theme::Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        if self.menu.is_none() {
            let targets = get_targets(&self.dependencies, render.clone())
                .await?
                .into_iter()
                .map(|target| -> Shared_menu_item<Option<Dependency>> {
                    Target_tree_item::new(target, self.working_directory.clone())
                        .into_shared()
                        .into()
                })
                .collect::<Vec<_>>();

            let Some(first_target) = targets.first() else {
                return Ok(vec![]);
            };

            let default_target = get_selector(first_target);
            let mut menu = Menu::new(targets, default_target);
            menu.set_submit_state(self.selected.clone());
            self.menu = Some(Widget_trait::into_shared(menu));
        }

        Ok(vec![display!(
            self.menu
                .clone()
                .expect("target menu must exist after initialization")
        )])
    }
}
