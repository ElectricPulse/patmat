pub mod target;
pub mod task;
mod ui;
pub mod utils;

pub use utils::normalize_path;

use color_eyre::eyre::Result;
use lucide_icons::Icon as Lucide_icon;
use std::path::{Path, PathBuf};
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
    state::Store,
    widget::{
        Layout_input, Shared_widget, Widget_trait,
        widgets::{
            icon::Icon,
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

impl Builder {
    pub fn working_directory(&self) -> &Path {
        &self.working_directory
    }

    pub fn title(&self) -> String {
        let normalized = normalize_path(&self.working_directory);
        format!("Patmat {normalized}")
    }
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

        let theme = theme.affect(render.clone()).await?;
        let mut name = Text::new(name_content);
        name.style.set(theme.specific.text.title);

        let name_item = Anchor::left(name);

        let path_paragraph_width = theme.units.em * 10.0;
        let paragraph_width = theme.units.em * 30.0;
        let label_style = theme.specific.text.paragraph.bold();

        let mut path_label = Text::new("Path:");
        path_label.style.set(label_style);
        let mut path_paragraph = Paragraph::new(Direction::Horizontal, path_paragraph_width);
        path_paragraph.set_styled_content(path, theme.specific.text.paragraph);
        let path_item = Anchor::left(Axis::new(
            Direction::Horizontal,
            (
                Anchor::middle(Icon::new(Lucide_icon::Folder)),
                Anchor::middle(path_label),
                Anchor::middle(path_paragraph),
            ),
        ));

        let mut status_label_text = Text::new("Status:");
        status_label_text.style.set(label_style);
        let mut status_paragraph = Paragraph::new(Direction::Horizontal, paragraph_width);
        status_paragraph.set_styled_content(status_label, theme.specific.text.paragraph);
        let status_item = Anchor::left(Axis::new(
            Direction::Horizontal,
            (
                Anchor::middle(Icon::new(status.get_icon())),
                Anchor::middle(status_label_text),
                Anchor::middle(status_paragraph),
            ),
        ));

        let metadata = if let Target_status::Error(error) = &status {
            let mut error_paragraph = Paragraph::new(Direction::Horizontal, theme.units.em * 25.0);
            error_paragraph.set_styled_content(format!("{error:#}"), theme.specific.text.paragraph);
            let mut error_title_label = Text::new("Error message:");
            error_title_label.style.set(label_style);
            let error_title = Anchor::left(Axis::new(
                Direction::Horizontal,
                (
                    Anchor::middle(Icon::new(Lucide_icon::AlertCircle)),
                    Anchor::middle(error_title_label),
                ),
            ));
            let error_body = Anchor::left(error_paragraph);
            Axis::new(
                Direction::Vertical,
                (name_item, path_item, status_item, error_title, error_body),
            )
        } else {
            Axis::new(Direction::Vertical, (name_item, path_item, status_item))
        };

        let widget = dependency.widget().affect(render.clone()).await?.clone();
        let mut panel = if let Some(widget) = widget {
            Axis::new(
                Direction::Vertical,
                (metadata, Linebreak::new(Direction::Horizontal), widget),
            )
        } else {
            Axis::new(Direction::Vertical, (metadata,))
        };

        panel.limit_cross = true;
        let panel = Anchor::top(panel);

        let axis = Axis::new(
            Direction::Horizontal,
            (self.target_tree.clone(), Linebreak::new(Direction::Vertical), panel),
        );

        Ok(vec![display!(axis)])
    }
}
