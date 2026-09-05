use async_trait::async_trait;
use color_eyre::eyre::Result;
use std::sync::{Arc, atomic::Ordering};
use drevo::{
    component::Children,
    geometry::{Direction, Rect},
    state::Store,
    widget::{
        LayoutInput, RenderInput, Widget, WidgetTrait,
        widgets::{layout::axis::Axis, paper::Paper, positioning::anchor::Anchor, text::Text},
    },
};
use drevo_macros::display;

#[cfg(test)]
mod tests;

const MAX_ROWS: usize = 3;
const MAX_NAME_CHARACTERS: usize = 64;
const MINIMUM_BAR_WIDTH: f64 = 360.0;
const BAR_HEIGHT: f64 = 12.0;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum CloneProgress {
    Starting,
    Running(Vec<ProgressRow>),
    Complete,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ProgressRow {
    name: String,
    value: String,
    fraction: Option<f64>,
}

impl CloneProgress {
    pub(super) fn from_tree(tree: &Arc<gix::progress::tree::Root>) -> Self {
        let mut tasks = Vec::new();
        tree.sorted_snapshot(&mut tasks);

        let mut rows = tasks
            .into_iter()
            .filter_map(|(_, task)| {
                let progress = task.progress?;
                let step = progress.step.load(Ordering::Relaxed);
                let value = match progress.unit {
                    Some(unit) => unit.display(step, progress.done_at, None).to_string(),
                    None => match progress.done_at {
                        Some(maximum) => format!("{step}/{maximum}"),
                        None => step.to_string(),
                    },
                };
                let fraction = progress
                    .done_at
                    .filter(|maximum| *maximum > 0)
                    .map(|maximum| (step as f64 / maximum as f64).clamp(0.0, 1.0));

                Some(ProgressRow {
                    name: single_line(&task.name),
                    value,
                    fraction,
                })
            })
            .collect::<Vec<_>>();

        if rows.len() > MAX_ROWS {
            rows = rows.split_off(rows.len() - MAX_ROWS);
        }

        Self::Running(rows)
    }
}

fn single_line(value: &str) -> String {
    let mut characters = value
        .chars()
        .map(|character| match character {
            '\n' | '\r' => ' ',
            character => character,
        })
        .take(MAX_NAME_CHARACTERS + 1)
        .collect::<String>();

    if characters.chars().count() > MAX_NAME_CHARACTERS {
        let _ = characters.pop();
        characters.push('…');
    }
    characters
}

#[derive(Clone)]
pub(crate) struct CloneProgressWidget {
    progress: Store<CloneProgress>,
}

impl CloneProgressWidget {
    pub(super) fn new(progress: Store<CloneProgress>) -> Self {
        Self { progress }
    }
}

#[async_trait]
impl WidgetTrait for CloneProgressWidget {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let progress = self.progress.affect(relayout.clone()).await?;
        let mut title = Text::new("Cloning repository");
        title
            .style
            .set(theme.affect(relayout).await?.specific.text.subtitle);
        let title_widget = Anchor::left(title);

        let content = match &*progress {
            CloneProgress::Starting => Axis::new(
                Direction::Vertical,
                (
                    title_widget,
                    Anchor::left(Text::new("0%")),
                    ProgressBar::new(0.0),
                ),
            ),
            CloneProgress::Running(rows) if rows.is_empty() => Axis::new(
                Direction::Vertical,
                (
                    title_widget,
                    Anchor::left(Text::new("0%")),
                    ProgressBar::new(0.0),
                ),
            ),
            CloneProgress::Running(rows) => {
                let mut children: Vec<Widget> = vec![title_widget.as_any()];
                for row in rows {
                    children.push(
                        Anchor::left(Text::new(format!("{}: {}", row.name, row.value))).as_any(),
                    );
                    if let Some(fraction) = row.fraction {
                        children.push(ProgressBar::new(fraction).as_any());
                    }
                }
                Axis::new(Direction::Vertical, children)
            }
            CloneProgress::Complete => Axis::new(
                Direction::Vertical,
                (
                    title_widget,
                    ProgressBar::new(1.0),
                    Anchor::left(Text::new("Clone complete")),
                ),
            ),
            CloneProgress::Failed(error) => Axis::new(
                Direction::Vertical,
                (
                    title_widget,
                    Anchor::left(Text::new(format!("Clone failed: {}", single_line(error)))),
                ),
            ),
        };

        Ok(vec![display!(Paper::new(content))])
    }
}

#[derive(Clone, Copy)]
struct ProgressBar {
    fraction: f64,
}

impl ProgressBar {
    fn new(fraction: f64) -> Self {
        Self {
            fraction: fraction.clamp(0.0, 1.0),
        }
    }
}

#[async_trait]
impl WidgetTrait for ProgressBar {
    async fn layout(
        &mut self,
        LayoutInput {
            hitbox, formula, ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        formula.constrain(
            drevo::id!(),
            drevo::constraint!(hitbox.get_dimension(Direction::Horizontal) >= MINIMUM_BAR_WIDTH),
        )?;
        hitbox
            .set_static_dimension(formula, Direction::Vertical, BAR_HEIGHT)
            .await?;
        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        RenderInput {
            rerender,
            theme,
            hitbox,
            scene,
            ..
        }: RenderInput<'_, '_>,
    ) -> Result<()> {
        let theme = theme.affect(rerender).await?;
        let radius = hitbox.size.height / 2.0;
        scene.fill_rounded_rect(hitbox, theme.semantic.border, radius);

        let fill = Rect::new(
            hitbox.origin.x,
            hitbox.origin.y,
            hitbox.size.width * self.fraction,
            hitbox.size.height,
        );
        scene.fill_rounded_rect(
            fill,
            theme.semantic.focus,
            radius.min(fill.size.width / 2.0),
        );
        Ok(())
    }
}
