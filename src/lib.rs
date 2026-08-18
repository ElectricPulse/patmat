pub mod target;
pub mod task;
mod ui;
mod utils;

use color_eyre::eyre::Result;
use std::path::PathBuf;
use vizual_macros::display;

use crate::{
    target::{Dependencies, Dependency, status::Target_status},
    ui::target_tree::Target_tree,
    utils::display_target_path,
};
use vizual::{
    self,
    component::Children,
    geometry::Direction,
    state::{State, Store},
    widget::{
        Layout_input, Shared_widget, Widget, Widget_trait,
        widgets::{
            layout::axis::Axis,
            linebreak::Linebreak,
            paragraph::Paragraph,
            positioning::anchor::Anchor,
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
        Layout_input {
            render,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let mut children: Vec<Widget> = vec![self.target_tree.clone().any()];

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
        let status = metadata.status.affect(render.clone()).await?.clone();
        let status_label = status.label();

        let theme = theme.affect(render).await?;
        let mut name = Paragraph::new(Direction::Horizontal, theme.units.em * 15.0);
        name.set_styled_content(name_content, theme.specific.text.title);

        let mut metadata_items: Vec<Widget> = vec![
            Anchor::left(name).any(),
            Anchor::left(Text::new(format!("Path: {path}"))).any(),
            Anchor::left(Text::new(format!("Status: {status_label}"))).any(),
        ];

        if let Target_status::Error(error) = &status {
            let mut error_paragraph = Paragraph::new(Direction::Horizontal, theme.units.em * 25.0);
            error_paragraph.set_content(format!("{error:#}"));
            metadata_items.push(Anchor::left(Text::new("Error message:")).any());
            metadata_items.push(Anchor::left(error_paragraph).any());
        }

        let metadata = Axis::new(Direction::Vertical, metadata_items);

        let panel: Vec<Widget> = if let Some(widget) = dependency.widget() {
            vec![
                metadata.any(),
                Linebreak::new(Direction::Horizontal).any(),
                widget,
            ]
        } else {
            vec![metadata.any()]
        };

        let mut panel = Axis::new(Direction::Vertical, panel);
        panel.limit_cross = true;
        let panel = Anchor::top(panel);
        children.push(Linebreak::new(Direction::Vertical).any());
        children.push(panel.any());

        let content = Axis::new(Direction::Horizontal, children);
        let working_directory = Anchor::left(Text::new(format!(
            "Working directory: {}",
            self.working_directory.display()
        )));
        let axis = Axis::new(
            Direction::Vertical,
            vec![working_directory.any(), content.any()],
        );

        Ok(vec![display!(axis)])
    }
}
