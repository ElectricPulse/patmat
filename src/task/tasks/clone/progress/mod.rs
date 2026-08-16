use arc_swap::ArcSwap;
use async_trait::async_trait;
use color_eyre::eyre::Result;
use std::sync::{Arc, atomic::Ordering};
use vizual::{
    component::{Children, Render_context, context::Component_context},
    geometry::{Direction, Rect},
    graphics::{scene::Scene, text::Text_context},
    layouter::hitbox::Hitbox,
    slot::manager::Slots,
    state::{State, Store},
    theme::Theme,
    widget::{
        Focus_provider, Widget, Widget_trait,
        widgets::{layout::axis::Axis, paper::Paper, positioning::anchor::Anchor, text::Text},
    },
};
use vizual_macros::display;

#[cfg(test)]
mod tests;

const MAX_ROWS: usize = 3;
const MAX_NAME_CHARACTERS: usize = 64;
const MINIMUM_BAR_WIDTH: f64 = 360.0;
const BAR_HEIGHT: f64 = 12.0;

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Clone_progress {
    Starting,
    Running(Vec<Progress_row>),
    Complete,
    Failed(String),
}

pub(super) type Clone_progress_state = Arc<ArcSwap<Clone_progress>>;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Progress_row {
    name: String,
    value: String,
    fraction: Option<f64>,
}

impl Clone_progress {
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

                Some(Progress_row {
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
pub(super) struct Clone_progress_widget {
    progress: Clone_progress_state,
}

impl Clone_progress_widget {
    pub(super) fn new(progress: Clone_progress_state) -> Self {
        Self { progress }
    }
}

#[async_trait]
impl Widget_trait for Clone_progress_widget {
    async fn layout(
        &mut self,
        render: vizual::Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        _hitbox: &mut Hitbox,
        _parent: Hitbox,
        _problem: Component_context,
        _text_context: &mut Text_context,
        slots: &mut Slots,
        _logical: &mut bool,
    ) -> Result<Children> {
        let progress = self.progress.load();
        let mut title = Text::new("Cloning repository");
        title
            .style
            .set(theme.affect(render).await?.specific.text.subtitle);
        let mut children: Vec<Widget> = vec![Box::new(Anchor::left(title))];

        match &**progress {
            Clone_progress::Starting => {
                children.push(Box::new(Anchor::left(Text::new("0%"))));
                children.push(Box::new(Progress_bar::new(0.0)));
            }
            Clone_progress::Running(rows) if rows.is_empty() => {
                children.push(Box::new(Anchor::left(Text::new("0%"))));
                children.push(Box::new(Progress_bar::new(0.0)));
            }
            Clone_progress::Running(rows) => {
                for row in rows {
                    children.push(Box::new(Anchor::left(Text::new(format!(
                        "{}: {}",
                        row.name, row.value
                    )))));
                    if let Some(fraction) = row.fraction {
                        children.push(Box::new(Progress_bar::new(fraction)));
                    }
                }
            }
            Clone_progress::Complete => {
                children.push(Box::new(Progress_bar::new(1.0)));
                children.push(Box::new(Anchor::left(Text::new("Clone complete"))));
            }
            Clone_progress::Failed(error) => {
                children.push(Box::new(Anchor::left(Text::new(format!(
                    "Clone failed: {}",
                    single_line(error)
                )))));
            }
        }

        let content = Axis::new(Direction::Vertical, children);
        Ok(vec![display!(Paper::new(content))])
    }
}

#[derive(Clone, Copy)]
struct Progress_bar {
    fraction: f64,
}

impl Progress_bar {
    fn new(fraction: f64) -> Self {
        Self {
            fraction: fraction.clamp(0.0, 1.0),
        }
    }
}

#[async_trait]
impl Widget_trait for Progress_bar {
    async fn layout(
        &mut self,
        _render: vizual::Render,
        _theme: Store<Theme>,
        _focus: &mut Focus_provider,
        hitbox: &mut Hitbox,
        _parent: Hitbox,
        problem: Component_context,
        _text_context: &mut Text_context,
        _slots: &mut Slots,
        _logical: &mut bool,
    ) -> Result<Children> {
        problem
            .constrain(vizual::constraint!(
                hitbox.get_dimension(Direction::Horizontal) >= MINIMUM_BAR_WIDTH
            ))
            .await?;
        hitbox
            .set_static_dimension(&problem, Direction::Vertical, BAR_HEIGHT)
            .await?;
        Ok(Vec::new())
    }

    async fn render(
        &mut self,
        render: vizual::Render,
        theme: Store<Theme>,
        _focus: &mut Focus_provider,
        hitbox: Rect,
        scene: &mut Scene<'_>,
        _text_context: &mut Text_context,
        _context: &Render_context<'_>,
    ) -> Result<()> {
        let theme = theme.affect(render).await?;
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
