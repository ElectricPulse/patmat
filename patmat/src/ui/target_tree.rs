use async_trait::async_trait;
use color_eyre::Result;
use derive_new::new;
use lucide_icons::Icon as LucideIcon;
use std::path::PathBuf;
use vizual::{
    component::Children,
    geometry::Direction,
    handlers::RetrieveHandler,
    state::{State, Store},
    widget::{
        LayoutInput, WidgetTrait,
        custom_widget::CustomWidgetTrait,
        widgets::{
            icon::Icon,
            layout::axis::Axis,
            menu::{Menu, MenuItem},
            positioning::anchor::Anchor,
            text::Text,
        },
    },
};
use vizual_macros::display;

use crate::target::{Dependencies, Dependency};
use crate::utils::{display_target_path, get_targets};

#[derive(Clone, new)]
struct TargetTreeItem {
    target: Dependency,
    working_directory: PathBuf,
}

#[async_trait::async_trait]
impl RetrieveHandler<Dependency> for TargetTreeItem {
    async fn on_retrieve(&mut self) -> Result<State<Dependency>> {
        Ok(self.target.clone().into())
    }
}

#[async_trait::async_trait]
impl CustomWidgetTrait for TargetTreeItem {
    type Payload = bool;

    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
        selected: bool,
    ) -> Result<Children> {
        let theme = theme.affect(relayout.clone()).await?;
        let metadata = self.target.get_metadata();

        let icon = Icon::new(metadata.status.affect(relayout.clone()).await?.get_icon());
        let icon = Anchor::v_middle(icon);

        let mut name = Text::new(metadata.name);
        let mut name_style = theme.specific.text.paragraph;
        if !selected {
            name_style.color = theme.semantic.text.muted;
        }
        name.style.set(name_style);
        let name = Anchor::left(name);

        let path = metadata.path.affect(relayout).await?.clone();
        let details = match path {
            Some(path) => {
                let path = display_target_path(&path, &self.working_directory);
                let mut path_text = Text::new(path);
                let mut path_style = theme.specific.text.paragraph;
                path_style.color = theme.semantic.text.muted;
                path_text.style.set(path_style);
                let mut folder_icon = Icon::new(LucideIcon::Folder);
                folder_icon.style.set(path_style);
                let path_row = Axis::new(
                    Direction::Horizontal,
                    (Anchor::v_middle(folder_icon), Anchor::v_middle(path_text)),
                );
                let path = Anchor::top_left(path_row);
                Axis::new(Direction::Vertical, (name, path))
            }
            None => Axis::new(Direction::Vertical, (name,)),
        };
        let details = Anchor::v_middle(details);

        let row = Axis::new(Direction::Horizontal, (details, icon));

        Ok(vec![display!(row)])
    }
}

#[derive(Clone)]
pub struct TargetTree {
    dependencies: Dependencies,
    selected: Store<Dependency>,
    selected_index: Store<usize>,
    working_directory: PathBuf,
}

impl TargetTree {
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
impl WidgetTrait for TargetTree {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout, slots, ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let targets = get_targets(&self.dependencies, relayout.clone())
            .await?
            .into_iter()
            .map(|target| -> MenuItem<Dependency> {
                Box::new(TargetTreeItem::new(target, self.working_directory.clone()))
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
