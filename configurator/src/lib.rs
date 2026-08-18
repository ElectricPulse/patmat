#![warn(rustdoc::broken_intra_doc_links)]
//! Tree-based configuration editing for Vizual applications.
//!
//! Implement [`Tree`] to describe editable fields and produce a serializable
//! configuration.

pub mod widgets;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use color_eyre::eyre::{Result, WrapErr, eyre};
use derive_where::derive_where;
use indexmap::IndexMap;
use serde::Serialize;
use vizual::{
    Vizual_msg,
    component::Children,
    event::{Event, Key_event, Pointer_event},
    geometry::Direction,
    handlers::{Retrieve_handler, Submit_handler},
    state::State,
    sync::{Mutex, Thread_safe},
    widget::{
        Layout_input, Render_input, Widget, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{
            button::Button,
            layout::{axis::Axis, grid::Grid},
            linebreak::Linebreak,
            menu::{Menu, Menu_item},
            positioning::{
                anchor::{Anchor, Anchors, Position as Anchor_position},
                space::Space,
            },
            text::Text,
            title_block::Title_block,
        },
    },
};
use vizual_macros::display;

#[async_trait]
/// Supplies the fields displayed by a [`Configurator`].
pub trait Tree: Thread_safe {
    type Configuration: Serialize + Thread_safe + Clone;

    fn get_tree(&self) -> Configuration_tree_branch;
    async fn create_config(&mut self) -> Result<Self::Configuration>;
}

#[async_trait]
/// A widget field that can return a configured value.
pub trait Field<Value>: Widget_trait + Retrieve_handler<Value> {}

dyn_clone::clone_trait_object!(<Value> Field<Value>);

impl<T, Value> Field<Value> for T
where
    T: Widget_trait + Retrieve_handler<Value> + Clone + 'static,
{
}

#[async_trait]
impl<Value: 'static> Widget_trait for Box<dyn Field<Value>> {
    async fn layout(&mut self, input: Layout_input<'_>) -> Result<Children> {
        (**self).layout(input).await
    }

    async fn render(&mut self, input: Render_input<'_, '_>) -> Result<()> {
        (**self).render(input).await
    }

    async fn on_all_events(&mut self, event: &Event) -> Result<Vizual_msg> {
        (**self).on_all_events(event).await
    }

    async fn on_mouse_click(&mut self, mouse: &Pointer_event) -> Result<Vizual_msg> {
        (**self).on_mouse_click(mouse).await
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        (**self).on_key_press(key).await
    }

    async fn on_other_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        (**self).on_other_event(event).await
    }

    async fn forward_event(&mut self, event: &Event) -> Result<Vizual_msg> {
        (**self).forward_event(event).await
    }
}

#[async_trait]
impl<Value: 'static> Retrieve_handler<Value> for Box<dyn Field<Value>> {
    async fn on_retrieve(&mut self) -> Result<Value> {
        (**self).on_retrieve().await
    }
}

/// An ordered group of configuration nodes.
pub struct Configuration_tree_branch(pub IndexMap<String, Configuration_tree>);

impl Configuration_tree_branch {
    fn get_node(self, cursor: &[String]) -> Result<Configuration_tree> {
        let mut node = Configuration_tree::Branch(self);

        for key in cursor {
            node = match node {
                Configuration_tree::Branch(mut branch) => branch
                    .0
                    .shift_remove(key)
                    .ok_or_else(|| eyre!("Expected key to exist"))?,
                Configuration_tree::Leaf(_) => return Err(eyre!("Expected branch")),
            };
        }

        Ok(node)
    }

    pub fn get_branch(self, cursor: &[String]) -> Result<Self> {
        self.get_node(cursor)?
            .into_branch()
            .map_err(|_| eyre!("Expected branch"))
    }

    pub fn get_leaf(self, cursor: &[String]) -> Result<Configuration_tree_leaf> {
        self.get_node(cursor)?
            .into_leaf()
            .map_err(|_| eyre!("Expected leaf"))
    }
}

/// A single editable configuration field.
pub struct Configuration_tree_leaf {
    pub widget: Widget,
    pub description: String,
    pub name: String,
}

/// A branch or editable leaf in a configuration tree.
pub enum Configuration_tree {
    Branch(Configuration_tree_branch),
    Leaf(Configuration_tree_leaf),
}

impl Configuration_tree {
    pub fn new_leaf(
        field: &(impl Widget_trait + Clone + 'static),
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self::Leaf(Configuration_tree_leaf {
            widget: field.clone().as_any(),
            description: description.into(),
            name: name.into(),
        })
    }

    fn into_branch(self) -> std::result::Result<Configuration_tree_branch, Self> {
        match self {
            Self::Branch(branch) => Ok(branch),
            value => Err(value),
        }
    }

    fn into_leaf(self) -> std::result::Result<Configuration_tree_leaf, Self> {
        match self {
            Self::Leaf(leaf) => Ok(leaf),
            value => Err(value),
        }
    }
}

#[derive(Clone)]
struct Tree_menu_item {
    name: String,
    cursor: Vec<String>,
    depth: usize,
}

#[async_trait]
impl Custom_widget_trait for Tree_menu_item {
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
        const INDENT: usize = 50;
        let theme = theme.affect(render).await?;
        let mut text = Text::new(&self.name);
        text.style.set(match selected {
            true => theme.specific.text.selected_subtitle,
            false => theme.specific.text.subtitle,
        });
        let mut button = Button::around(text);
        button.highlighted = selected;
        let button = Space::left(button, (INDENT * self.depth) as f64, 1);
        Ok(vec![display!(button)])
    }
}

#[async_trait]
impl Retrieve_handler<Vec<String>> for Tree_menu_item {
    async fn on_retrieve(&mut self) -> Result<Vec<String>> {
        Ok(self.cursor.clone())
    }
}

fn collect_menu_items(
    branch: &Configuration_tree_branch,
    cursor: &[String],
    items: &mut Vec<Menu_item<Vec<String>>>,
) {
    for (name, child) in &branch.0 {
        let mut child_cursor = cursor.to_vec();
        child_cursor.push(name.clone());
        let depth = cursor.len();

        items.push(Box::new(Tree_menu_item {
            name: name.clone(),
            cursor: child_cursor.clone(),
            depth,
        }));

        if let Configuration_tree::Branch(branch) = child {
            collect_menu_items(branch, &child_cursor, items);
        }
    }
}

/// A widget editor for a [`Tree`].
#[derive_where(Clone)]
pub struct Configurator<Tree: crate::Tree> {
    tree: Arc<Mutex<Tree>>,
    menu: Menu<Vec<String>>,
    submit_handler: Option<Box<dyn Submit_handler<Tree::Configuration>>>,
    configuration_path: PathBuf,
}

#[async_trait]
impl<Tree: crate::Tree> Submit_handler<bool> for Configurator<Tree> {
    async fn on_submit(&mut self, _focused: bool) -> Result<Vizual_msg> {
        let config = self.tree.lock().await?.create_config().await?;
        let string =
            serde_saphyr::to_string(&config).wrap_err("Failed to serialize configuration")?;
        fs::write(&self.configuration_path, string).wrap_err("Failed to save configuration")?;
        println!("Configuration saved to {}", self.configuration_path.display());

        if let Some(submit_handler) = &mut self.submit_handler {
            return submit_handler.on_submit(config).await;
        }

        Vizual_msg::none()
    }
}

pub async fn new<Tree: crate::Tree>(
    configuration_path: impl AsRef<Path>,
    tree: Tree,
    submit_handler: Option<impl Submit_handler<Tree::Configuration>>,
) -> Result<Configurator<Tree>> {
    let tree_branch = tree.get_tree();
    let mut items = Vec::new();
    collect_menu_items(&tree_branch, &[], &mut items);

    if items.is_empty() {
        return Err(eyre!("Expected at least one configuration item"));
    }

    let mut menu = Menu::new(items, 0).await?;
    menu.item_block = false;
    let tree = Arc::new(Mutex::new(tree));

    Ok(Configurator {
        tree,
        menu,
        configuration_path: configuration_path.as_ref().to_owned(),
        submit_handler: submit_handler
            .map(|handler| Box::new(handler) as Box<dyn Submit_handler<Tree::Configuration>>),
    })
}

#[async_trait]
impl<Tree: crate::Tree> Widget_trait for Configurator<Tree> {
    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            slots,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let cursor = self
            .menu
            .submitted
            .affect(render.clone())
            .await?
            .clone();

        //TODO: this menu could later be moved into the menu item of the tree to make it clearer
        let field: Option<Widget> = {
            let tree = self.tree.lock().await?;

            if let Ok(leaf) = tree.get_tree().get_leaf(&cursor) {
                let description = Text::new(leaf.description);
                let description = Anchor::left(description);

                let axis = Axis::new(
                    Direction::Vertical,
                    (
                        description,
                        Linebreak::new(Direction::Horizontal),
                        leaf.widget,
                    ),
                );

                let leaf = Title_block::new(axis, leaf.name);

                Some(leaf.as_any())
            } else {
                None
            }
        };

        let theme = theme.affect(render).await?;
        let gap = theme.semantic.axis.gap;
        let menu_block = Title_block::new(self.menu.clone(), "Config");
        let menu_view = Anchor::new(
            menu_block,
            Anchors {
                horizontal: Some(Anchor_position::Start),
                vertical: Some(Anchor_position::Start),
            },
        );

        let mut text = Text::new("Apply");
        text.style.set(theme.specific.text.selected_subtitle);
        let button = Button::new(text, self.clone());
        let button = Anchor::new(
            button,
            Anchors {
                horizontal: Some(Anchor_position::End),
                vertical: Some(Anchor_position::End),
            },
        );

        let grid = if let Some(field) = field {
            let field = Anchor::new(
                field,
                Anchors {
                    horizontal: Some(Anchor_position::End),
                    vertical: Some(Anchor_position::Start),
                },
            );
            Grid::new((menu_view, field, button), gap)
        } else {
            Grid::new((menu_view, button), gap)
        };

        Ok(vec![display!(grid)])
    }
}
