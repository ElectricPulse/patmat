pub mod target;
pub mod task;
mod ui;
pub mod utils;

use color_eyre::eyre::Result;
use lucide_icons::Icon as LucideIcon;
use std::path::{Path, PathBuf};
use vizual_macros::display;

use crate::{
    target::{Dependencies, Dependency, status::TargetStatus},
    ui::target_tree::TargetTree,
    utils::display_target_path,
};
use vizual::{
    self,
    component::Children,
    geometry::Direction,
    state::Store,
    utils::normalize_path,
    widget::{
        LayoutInput, SharedWidget, Widget, WidgetTrait,
        widgets::{
            icon::Icon, layout::axis::Axis, linebreak::Linebreak, paragraph::Paragraph,
            positioning::anchor::Anchor, scroll::Scroll, text::Text,
        },
    },
};

#[derive(Clone)]
pub struct Builder {
    target_tree: SharedWidget<Scroll>,
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
    let target_tree = TargetTree::new(
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
impl WidgetTrait for Builder {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let dependency = &*self.selected_dependency.affect(relayout.clone()).await?;
        let metadata = dependency.get_metadata();
        let name_content = metadata.name.affect(relayout.clone()).await?.clone();
        let path = metadata
            .path
            .affect(relayout.clone())
            .await?
            .as_ref()
            .map_or_else(
                || "None".to_owned(),
                |path| display_target_path(path, &self.working_directory),
            );
        let status = metadata.status.affect(relayout.clone()).await?.clone();
        let status_label = status.label();

        let theme = theme.affect(relayout.clone()).await?;
        let name_item = Anchor::left(Text::new(name_content).style(theme.specific.text.title));

        let path_paragraph_width = theme.units.em * 10.0;
        let paragraph_width = theme.units.em * 15.0;
        let label_style = theme.specific.text.paragraph.bold();

        let mut path_paragraph = Paragraph::new(Direction::Horizontal, path_paragraph_width);
        path_paragraph.set_styled_content(path, theme.specific.text.paragraph);
        let path_item = Anchor::left(Axis::new(
            Direction::Horizontal,
            (
                Anchor::v_middle(Icon::new(LucideIcon::Folder)),
                Anchor::v_middle(Text::new("Path:").style(label_style)),
                Anchor::v_middle(path_paragraph),
            ),
        ));

        let mut status_paragraph = Paragraph::new(Direction::Horizontal, paragraph_width);
        status_paragraph.set_styled_content(status_label, theme.specific.text.paragraph);
        let status_item = Anchor::left(Axis::new(
            Direction::Horizontal,
            (
                Anchor::v_middle(Icon::new(status.get_icon())),
                Anchor::v_middle(Text::new("Status:").style(label_style)),
                Anchor::v_middle(status_paragraph),
            ),
        ));

        let metadata = if let TargetStatus::Error(error) = &status {
            let mut error_paragraph = Paragraph::new(Direction::Horizontal, theme.units.em * 25.0);
            error_paragraph.set_styled_content(format!("{error:#}"), theme.specific.text.paragraph);
            let error_title = Anchor::left(Axis::new(
                Direction::Horizontal,
                (
                    Anchor::v_middle(Icon::new(LucideIcon::AlertCircle)),
                    Anchor::v_middle(Text::new("Error message:").style(label_style)),
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

        let widget = dependency.widget().affect(relayout.clone()).await?.clone();
        let panel: Widget = if let Some(widget) = widget {
            Box::new(Axis::new(
                Direction::Vertical,
                (
                    Anchor::top_left(metadata),
                    Linebreak::new(Direction::Horizontal),
                    widget,
                ),
            ))
        } else {
            Box::new(Anchor::top_right(Axis::new(
                Direction::Vertical,
                (metadata,),
            )))
        };

        let axis = Axis::new(
            Direction::Horizontal,
            (
                self.target_tree.clone(),
                Linebreak::new(Direction::Vertical),
                panel,
            ),
        );

        Ok(vec![display!(axis)])
    }
}
