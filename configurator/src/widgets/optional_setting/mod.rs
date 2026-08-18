use std::{marker::PhantomData, sync::Arc};
use async_trait::async_trait;
use color_eyre::eyre::Result;
use derive_where::derive_where;
use vizual::geometry::Direction;
use vizual::{
    Render,
    component::Children,
    handlers::Retrieve_handler,
    state::{Constant, Read_guard, State, State_trait},
    sync::Thread_safe,
    widget::{
        Layout_input, Widget_trait,
        custom_widget::Custom_widget_trait,
        widgets::{
            layout::axis::Axis,
            menu::{Menu, Menu_item},
            positioning::anchor::Anchor,
            text::Text,
        },
    },
};
use vizual_macros::display;

use crate::Field;

#[derive(Clone)]
struct Some_state<Value: Thread_safe>(State<Value>);

#[async_trait]
impl<Value: Thread_safe + Clone> State_trait for Some_state<Value> {
    type Output = Option<Value>;

    async fn read(&self) -> Result<Read_guard<Self::Output>> {
        let guard = self.0.read().await?;
        Ok(Read_guard::new(Arc::new(Some((*guard).clone()))))
    }

    async fn affect(&self, signal: Render) -> Result<Read_guard<Self::Output>> {
        let guard = self.0.affect(signal).await?;
        Ok(Read_guard::new(Arc::new(Some((*guard).clone()))))
    }
}

#[derive_where(Clone)]
struct Default_leaf_value<Value: Thread_safe> {
    label: String,
    value: PhantomData<Value>,
}

#[async_trait]
impl<Value: Thread_safe> Retrieve_handler<Option<Value>> for Default_leaf_value<Value> {
    async fn on_retrieve(&mut self) -> Result<State<Option<Value>>> {
        Ok(Constant::from(None).into())
    }
}

#[async_trait]
impl<Value: Thread_safe> Custom_widget_trait for Default_leaf_value<Value> {
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
        let theme = theme.affect(render).await?;
        let mut text = Text::new(format!("Default - {}", self.label));
        let mut style = theme.specific.text.button;
        if !selected {
            style.color = theme.semantic.text.muted;
        }
        text.style.set(style);

        Ok(vec![display!(text)])
    }
}

#[derive_where(Clone)]
struct Custom_leaf_value<Value: Thread_safe> {
    field: Box<dyn Field<Value>>,
}

#[async_trait]
impl<Value: Thread_safe + Clone> Retrieve_handler<Option<Value>> for Custom_leaf_value<Value> {
    async fn on_retrieve(&mut self) -> Result<State<Option<Value>>> {
        let inner_state = self.field.on_retrieve().await?;
        Ok(Box::new(Some_state(inner_state)))
    }
}

#[async_trait]
impl<Value: Thread_safe> Custom_widget_trait for Custom_leaf_value<Value> {
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
        let theme = theme.affect(render).await?;
        let mut title = Text::new("Custom");
        let mut style = theme.specific.text.button;
        if !selected {
            style.color = theme.semantic.text.muted;
        }
        title.style.set(style);

        if !selected {
            return Ok(vec![display!(title)]);
        }

        let title = Anchor::left(title);
        let field = Anchor::left(self.field.clone());

        let axis = Axis::new(Direction::Vertical, (title, field));

        Ok(vec![display!(axis)])
    }
}

#[derive_where(Clone)]
pub struct Optional_setting<Value: Clone + Thread_safe> {
    menu: Menu<Option<Value>>,
}

impl<Value: Clone + Thread_safe> Optional_setting<Value> {
    pub async fn new(
        default_value: impl Into<String>,
        is_default: bool,
        field: impl Field<Value> + 'static,
    ) -> Result<Self> {
        let default_item: Menu_item<Option<Value>> = Box::new(Default_leaf_value {
            label: default_value.into(),
            value: PhantomData,
        });
        let custom_item: Menu_item<Option<Value>> = Box::new(Custom_leaf_value {
            field: Box::new(field),
        });
        let items = vec![default_item, custom_item];
        let default_index = usize::from(!is_default);
        let menu = Menu::new(items, default_index).await?;
        Ok(Self { menu })
    }

    pub async fn set_is_default(&mut self, is_default: bool) -> Result<()> {
        self.menu.set_index(usize::from(!is_default)).await
    }
}

#[async_trait]
impl<Value: Clone + Thread_safe> Widget_trait for Optional_setting<Value> {
    async fn layout(
        &mut self,
        Layout_input { slots, .. }: Layout_input<'_>,
    ) -> Result<Children> {
        Ok(vec![display!(self.menu.clone())])
    }
}

#[async_trait]
impl<Value: Clone + Thread_safe> Retrieve_handler<Option<Value>> for Optional_setting<Value> {
    async fn on_retrieve(&mut self) -> Result<State<Option<Value>>> {
        self.menu.on_retrieve().await
    }
}

#[cfg(test)]
mod tests;
