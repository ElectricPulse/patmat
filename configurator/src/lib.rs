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
    Vizual_command, Vizual_msg, check_quit_event,
    component::Children,
    event::{Event, Key_event, Pointer_event},
    geometry::Direction,
    handlers::{Retrieve_handler, Submit_handler},
    slot::Component_slot,
    state::State,
    sync::{Mutex, Thread_safe},
    widget::{
        Layout_input, Render_input, Shared_widget, Widget, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{
            button::Button,
            layout::{axis::Axis, grid::Grid},
            linebreak::Linebreak,
            menu::{Menu, Menu_item},
            popup::Popup,
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
    type Configuration: Serialize;

    fn get_tree(&self) -> Configuration_tree_branch;
    async fn create_config(&mut self) -> Result<Self::Configuration>;
}

#[async_trait]
/// A widget field that can return an optional configured value.
pub trait Field<Value>: Widget_trait + Retrieve_handler<Option<Value>> {}

dyn_clone::clone_trait_object!(<Value> Field<Value>);

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
    pub fn new_leaf<T: Widget_trait>(
        field: &Shared_widget<T>,
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self::Leaf(Configuration_tree_leaf {
            widget: field.clone().into(),
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
        let text = Space::left(text, (INDENT * self.depth) as f64, 1);
        Ok(vec![display!(text)])
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

#[derive_where(Clone)]
pub struct Config_manager<T: Tree> {
    tree: Arc<Mutex<T>>,
    configuration_path: PathBuf,
    submit_handler: Box<dyn Submit_handler<bool>>,
}

#[derive_where(Clone)]
struct Config_manager_handle<T: Tree> {
    manager: Arc<Mutex<Config_manager<T>>>,
}

/// A widget editor for a [`Tree`].
#[derive_where(Clone)]
pub struct Configurator<T: Tree> {
    tree: Arc<Mutex<T>>,
    menu: Shared_widget<Menu<Vec<String>>>,
    config_manager: Config_manager_handle<T>,
    submit: Shared_widget<Popup>,
    submitting: bool,
    popup_slot: Component_slot,
}

impl<T: Tree> Config_manager<T> {
    async fn save(&mut self) -> Result<()> {
        let config = self.tree.lock().await?.create_config().await?;
        let string =
            serde_saphyr::to_string(&config).wrap_err("Failed to serialize configuration")?;
        fs::write(&self.configuration_path, string).wrap_err("Failed to save configuration")?;
        Ok(())
    }

    async fn complete(&mut self, should_save: bool) -> Result<Vizual_msg> {
        self.submit_handler.on_submit(should_save).await
    }
}

#[async_trait]
impl<T: Tree> Submit_handler<bool> for Config_manager_handle<T> {
    async fn on_submit(&mut self, should_save: bool) -> Result<Vizual_msg> {
        let mut manager = self.manager.lock().await?;

        if should_save {
            manager.save().await?;
        }

        manager.complete(should_save).await
    }
}

#[async_trait]
impl<T: Tree> Submit_handler<String> for Config_manager_handle<T> {
    async fn on_submit(&mut self, _label: String) -> Result<Vizual_msg> {
        self.manager.lock().await?.save().await?;
        Vizual_msg::new(Vizual_command::Layout)
    }
}

/// Creates a configurator that optionally saves YAML to `configuration_path`.
pub async fn configurator<T: Tree>(
    configuration_path: impl AsRef<Path>,
    tree: T,
    submit_handler: impl Submit_handler<bool>,
) -> Result<Configurator<T>> {
    let tree_branch = tree.get_tree();
    let mut items = Vec::new();
    collect_menu_items(&tree_branch, &[], &mut items);

    if items.is_empty() {
        return Err(eyre!("Expected at least one configuration item"));
    }

    let menu = Menu::new(items, 0).await?.into_shared();
    let tree = Arc::new(Mutex::new(tree));

    let config_manager = Config_manager_handle {
        manager: Arc::new(Mutex::new(Config_manager {
            tree: tree.clone(),
            configuration_path: configuration_path.as_ref().to_owned(),
            submit_handler: Box::new(submit_handler) as Box<dyn Submit_handler<bool>>,
        })),
    };

    Ok(Configurator {
        tree,
        menu,
        config_manager: config_manager.clone(),
        submit: Popup::new(config_manager).await?.into_shared(),
        submitting: false,
        popup_slot: Component_slot::new(),
    })
}

#[async_trait]
impl<T: Tree> Widget_trait for Configurator<T> {
    async fn layout(
        &mut self,
        Layout_input {
            render,
            theme,
            hitbox,
            problem,
            slots,
            root,
            ..
        }: Layout_input<'_>,
    ) -> Result<Children> {
        let cursor = self
            .menu
            .lock()
            .await?
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
                    vec![
                        description.any(),
                        Linebreak::new(Direction::Horizontal).any(),
                        leaf.widget,
                    ],
                );

                let leaf = Title_block::new(axis, leaf.name);

                Some(leaf.any())
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
        let mut children: Vec<Widget> = vec![menu_view.any()];

        if let Some(field) = field {
            let field = Anchor::new(
                field,
                Anchors {
                    horizontal: Some(Anchor_position::End),
                    vertical: Some(Anchor_position::Start),
                },
            );

            children.push(field.any());
        }

        let mut text = Text::new("Apply");
        text.style.set(theme.specific.text.selected_subtitle);
        let button = Button::new(text, self.config_manager.clone());
        let button = Anchor::new(
            button,
            Anchors {
                horizontal: Some(Anchor_position::End),
                vertical: Some(Anchor_position::End),
            },
        );

        children.push(button.any());

        let grid = Grid::new(children, gap);

        if self.submitting {
            let popup = self
                .popup_slot
                .set_child(self.submit.clone(), problem.clone(), hitbox)
                .await?;

            popup.lock().await?.logical = true;
            root.lock().await?.children.push(popup.clone());
            return Ok(vec![display!(grid), popup]);
        }

        Ok(vec![display!(grid)])
    }

    async fn on_key_press(&mut self, key: &Key_event) -> Result<Vizual_msg> {
        if check_quit_event(key) {
            if !self.submitting {
                self.submitting = true;

                return Vizual_msg::new(Vizual_command::Focus(self.popup_slot.get_reference()));
            }

            return Vizual_msg::none();
        }

        Vizual_msg::none()
    }
}
