pub mod target;
pub mod task;
mod ui;
mod utils;

use color_eyre::eyre::Result;
use std::path::PathBuf;
use vizual_macros::display;

use crate::{
    target::{Dependencies, Dependency},
    ui::target_tree::Target_tree,
    utils::display_target_path,
};
use vizual::{
    self,
    component::{Children, context::Component_context},
    geometry::Direction,
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    widget::{
        Focus_provider, Shared_widget, Widget, Widget_trait,
        widgets::{
            layout::axis::Axis,
            linebreak::Linebreak,
            paragraph::Paragraph,
            positioning::anchor::{Anchor, Anchors, Position},
            scroll::Scroll,
            text::Text,
        },
    },
};

#[derive(Clone)]
pub struct Builder {
    target_tree: Shared_widget<Scroll>,
    selected_dependency: Store<Dependency>,
    working_directory: PathBuf,
}

pub fn new(dependencies: Dependencies, working_directory: PathBuf) -> Builder {
    let targets = dependencies.clone();

    let _ = tokio::spawn(async move {
        for target in targets {
            let _ = target.ensure_ran().await;
        }
    });

    let selected_dependency = Store::new(
        dependencies
            .first()
            .cloned()
            .expect("Dependencies must not be empty"),
    );
    let target_tree = Target_tree::new(
        dependencies,
        selected_dependency.clone(),
        working_directory.clone(),
    );
    let target_tree = Scroll::new(target_tree).into_shared();

    Builder {
        target_tree,
        selected_dependency,
        working_directory,
    }
}

#[async_trait::async_trait]
impl Widget_trait for Builder {
    async fn layout(
        &mut self,
        render: vizual::Render,
        theme: vizual::state::Store<vizual::theme::Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut vizual::graphics::text::Text_context,
        slots: &mut Slots,
    ) -> Result<Children> {
        let mut children: Vec<Widget> = vec![Box::new(self.target_tree.clone())];

        let dependency = &*self.selected_dependency.affect(render.clone()).await?;
        let metadata = dependency.get_metadata();
        let name_content = metadata.name.affect(render.clone()).await?.clone();
        let path = metadata
            .path
            .affect(render.clone())
            .await?
            .as_ref()
            .map_or_else(
                || "None".to_owned(),
                |path| display_target_path(path, &self.working_directory),
            );
        let status = metadata.status.affect(render.clone()).await?.label();

        let theme = theme.affect(render).await?;
        let mut name = Paragraph::new(Direction::Horizontal, theme.units.em * 15.0);
        name.set_styled_content(name_content, theme.specific.text.title);

        let metadata = Axis::new(
            Direction::Vertical,
            vec![
                Box::new(Anchor::left(name)),
                Box::new(Anchor::left(Text::new(format!("Path: {path}")))),
                Box::new(Anchor::left(Text::new(format!("Status: {status}")))),
            ],
        );
        let mut panel: Vec<Widget> = vec![Box::new(metadata)];
        if let Some(widget) = dependency.widget() {
            panel.push(Box::new(Linebreak::new(Direction::Horizontal)));
            panel.push(widget);
        }

        let panel = Axis::new(Direction::Vertical, panel);
        children.push(Box::new(Linebreak::new(Direction::Vertical)));
        children.push(Box::new(Anchor::new(
            panel,
            Anchors {
                horizontal: None,
                vertical: Some(Position::Start),
            },
        )));

        let content = Axis::new(Direction::Horizontal, children);
        let working_directory = Anchor::left(Text::new(format!(
            "Working directory: {}",
            self.working_directory.display()
        )));
        let axis = Axis::new(
            Direction::Vertical,
            vec![Box::new(working_directory), Box::new(content)],
        );

        Ok(vec![display!(axis)])
    }
}
