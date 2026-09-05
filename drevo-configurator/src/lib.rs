pub mod widgets;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use async_trait::async_trait;
use color_eyre::eyre::{Context, Result, eyre};
use derive_where::derive_where;
use indexmap::IndexMap;
use serde::Serialize;
use drevo_macros::display;

use drevo::{
    VizualMsg,
    component::Children,
    event::Event,
    geometry::Direction,
    handlers::{RetrieveHandler, SubmitHandler},
    state::State,
    sync::{Mutex, ThreadSafe},
    widget::{
        AllEvents, KeyPress, LayoutInput, MouseEvent, OtherEvent, RenderInput, Widget, WidgetTrait,
        custom_widget::CustomWidgetTrait,
        widgets::{
            button::Button,
            layout::{axis::Axis, grid::Grid},
            linebreak::Linebreak,
            menu::{Menu, MenuItem},
            positioning::{
                anchor::{Anchor, AnchorPosition, Anchors},
                space::Space,
            },
            text::Text,
            title_block::TitleBlock,
        },
    },
};

/// A configuration hierarchy capable of generating widgets and parsing output.
#[async_trait]
pub trait Tree: ThreadSafe {
    type Configuration: Serialize + ThreadSafe + Clone;

    fn get_tree(&self) -> ConfigurationTreeBranch;
    async fn create_config(&mut self) -> Result<Self::Configuration>;
}

#[async_trait]
/// A widget field that can return a configured value.
pub trait Field<Value: ThreadSafe>: WidgetTrait + RetrieveHandler<Value> {}

dyn_clone::clone_trait_object!(<Value> Field<Value> where Value: ThreadSafe);

impl<T, Value: ThreadSafe> Field<Value> for T where
    T: WidgetTrait + RetrieveHandler<Value> + Clone + 'static
{
}

#[async_trait]
impl<Value: ThreadSafe + 'static> WidgetTrait for Box<dyn Field<Value>> {
    async fn layout(&mut self, input: LayoutInput<'_>) -> Result<Children> {
        (**self).layout(input).await
    }

    async fn render(&mut self, input: RenderInput<'_, '_>) -> Result<()> {
        (**self).render(input).await
    }

    async fn on_all_events(&mut self, input: AllEvents<'_>) -> Result<VizualMsg> {
        (**self).on_all_events(input).await
    }

    async fn on_mouse_click(&mut self, input: MouseEvent<'_>) -> Result<VizualMsg> {
        (**self).on_mouse_click(input).await
    }

    async fn on_key_press(&mut self, input: KeyPress<'_>) -> Result<VizualMsg> {
        (**self).on_key_press(input).await
    }

    async fn on_other_event(&mut self, input: OtherEvent<'_>) -> Result<VizualMsg> {
        (**self).on_other_event(input).await
    }

    async fn forward_event(
        &mut self,
        event: &Event,
        relayout: drevo::Signal,
        window: std::sync::Arc<drevo::Window>,
    ) -> Result<VizualMsg> {
        (**self).forward_event(event, relayout, window).await
    }
}

#[async_trait]
impl<Value: ThreadSafe + 'static> RetrieveHandler<Value> for Box<dyn Field<Value>> {
    async fn on_retrieve(&mut self) -> Result<State<Value>> {
        (**self).on_retrieve().await
    }
}

/// An ordered group of configuration nodes.
pub struct ConfigurationTreeBranch(pub IndexMap<String, ConfigurationTree>);

impl ConfigurationTreeBranch {
    fn get_node(self, cursor: &[String]) -> Result<ConfigurationTree> {
        let mut node = ConfigurationTree::Branch(self);

        for key in cursor {
            node = match node {
                ConfigurationTree::Branch(mut branch) => branch
                    .0
                    .shift_remove(key)
                    .ok_or_else(|| eyre!("Expected key to exist"))?,
                ConfigurationTree::Leaf(_) => return Err(eyre!("Expected branch")),
            };
        }

        Ok(node)
    }

    pub fn get_leaf(self, cursor: &[String]) -> Result<ConfigurationTreeLeaf> {
        match self.get_node(cursor)? {
            ConfigurationTree::Leaf(leaf) => Ok(leaf),
            ConfigurationTree::Branch(_) => Err(eyre!("Expected leaf")),
        }
    }

    pub fn get_branch(self, cursor: &[String]) -> Result<ConfigurationTreeBranch> {
        match self.get_node(cursor)? {
            ConfigurationTree::Branch(branch) => Ok(branch),
            ConfigurationTree::Leaf(_) => Err(eyre!("Expected branch")),
        }
    }
}

/// A single editable configuration field.
pub struct ConfigurationTreeLeaf {
    pub widget: Widget,
    pub description: String,
    pub name: String,
}

/// A branch or editable leaf in a configuration tree.
pub enum ConfigurationTree {
    Branch(ConfigurationTreeBranch),
    Leaf(ConfigurationTreeLeaf),
}

impl ConfigurationTree {
    pub fn new_leaf(
        field: &(impl WidgetTrait + Clone + 'static),
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self::Leaf(ConfigurationTreeLeaf {
            widget: field.clone().as_any(),
            description: description.into(),
            name: name.into(),
        })
    }
}

#[derive(Clone)]
struct TreeMenuItem {
    name: String,
    cursor: Vec<String>,
    depth: usize,
}

#[async_trait]
impl CustomWidgetTrait for TreeMenuItem {
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
        const INDENT: usize = 50;
        let theme = theme.affect(relayout).await?;
        let mut style = theme.specific.text.paragraph;
        if !selected {
            style.color = theme.semantic.text.muted;
        }
        let mut button = Button::around(Text::new(&self.name).style(style));
        button.highlighted = selected;
        let button = Space::left(button, (INDENT * self.depth) as f64, 1);
        Ok(vec![display!(button)])
    }
}

#[async_trait]
impl RetrieveHandler<Vec<String>> for TreeMenuItem {
    async fn on_retrieve(&mut self) -> Result<State<Vec<String>>> {
        Ok(self.cursor.clone().into())
    }
}

fn collect_menu_items(
    branch: &ConfigurationTreeBranch,
    cursor: &[String],
    items: &mut Vec<MenuItem<Vec<String>>>,
) {
    for (name, child) in &branch.0 {
        let mut child_cursor = cursor.to_vec();
        child_cursor.push(name.clone());
        let depth = cursor.len();

        items.push(Box::new(TreeMenuItem {
            name: name.clone(),
            cursor: child_cursor.clone(),
            depth,
        }));

        if let ConfigurationTree::Branch(branch) = child {
            collect_menu_items(branch, &child_cursor, items);
        }
    }
}

/// A widget editor for a [`Tree`].
#[derive_where(Clone)]
pub struct Configurator<Tree: crate::Tree> {
    tree: Arc<Mutex<Tree>>,
    menu: Menu<Vec<String>>,
    submit_handler: Option<Box<dyn SubmitHandler<Tree::Configuration>>>,
    configuration_path: PathBuf,
}

#[async_trait]
impl<Tree: crate::Tree> SubmitHandler<bool> for Configurator<Tree> {
    async fn on_submit(&mut self, _focused: bool) -> Result<VizualMsg> {
        let config = self.tree.lock().await?.create_config().await?;
        let string =
            serde_saphyr::to_string(&config).wrap_err("Failed to serialize configuration")?;

        println!(
            "Saving configuration to {}",
            self.configuration_path.display()
        );

        fs::write(&self.configuration_path, string).wrap_err("Failed to save configuration")?;

        if let Some(submit_handler) = &mut self.submit_handler {
            return submit_handler.on_submit(config).await;
        }

        VizualMsg::none()
    }
}

pub async fn new<Tree: crate::Tree>(
    configuration_path: impl AsRef<Path>,
    tree: Tree,
    submit_handler: Option<impl SubmitHandler<Tree::Configuration>>,
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
            .map(|handler| Box::new(handler) as Box<dyn SubmitHandler<Tree::Configuration>>),
    })
}

#[async_trait]
impl<Tree: crate::Tree> WidgetTrait for Configurator<Tree> {
    async fn layout(
        &mut self,
        LayoutInput {
            relayout,
            theme,
            slots,
            ..
        }: LayoutInput<'_>,
    ) -> Result<Children> {
        let cursor = self
            .menu
            .on_retrieve()
            .await?
            .affect(relayout.clone())
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

                let leaf = TitleBlock::new(axis, leaf.name);

                Some(leaf.as_any())
            } else {
                None
            }
        };

        let theme = theme.affect(relayout).await?;
        let gap = theme.semantic.axis.gap;
        let menu = TitleBlock::new(self.menu.clone(), "Config");
        let menu = Anchor::top_left(menu);

        let button = Button::new(
            Text::new("Apply").style(theme.specific.text.button),
            self.clone(),
        );
        let button = Anchor::new(
            button,
            Anchors {
                horizontal: Some(AnchorPosition::End),
                vertical: Some(AnchorPosition::End),
            },
        );

        let grid = if let Some(field) = field {
            let field = Anchor::new(
                field,
                Anchors {
                    horizontal: Some(AnchorPosition::End),
                    vertical: Some(AnchorPosition::Start),
                },
            );
            Grid::new((menu, field, button), gap)
        } else {
            Grid::new((menu, button), gap)
        };

        Ok(vec![display!(grid)])
    }
}
