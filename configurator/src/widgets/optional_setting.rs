use async_trait::async_trait;
use color_eyre::eyre::{Result, eyre};
use derive_where::derive_where;
use std::marker::PhantomData;
use vizual::geometry::Direction;
use vizual::{
    component::Children,
    handlers::Retrieve_handler,
    state::State,
    sync::Thread_safe,
    widget::{
        Layout_input, Shared_widget, Widget_trait,
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

#[derive_where(Clone)]
struct Default_leaf_value<Value: Thread_safe> {
    label: String,
    value: PhantomData<Value>,
}

#[async_trait]
impl<Value: Thread_safe> Retrieve_handler<Option<Value>> for Default_leaf_value<Value> {
    async fn on_retrieve(&mut self) -> Result<Option<Value>> {
        Ok(None)
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
        text.style.set(match selected {
            true => theme.specific.text.selected_subtitle,
            false => theme.specific.text.subtitle,
        });

        Ok(vec![display!(text)])
    }
}

#[derive_where(Clone)]
struct Custom_leaf_value<Value: Thread_safe> {
    field: Box<dyn Field<Value>>,
}

#[async_trait]
impl<Value: Thread_safe> Retrieve_handler<Option<Value>> for Custom_leaf_value<Value> {
    async fn on_retrieve(&mut self) -> Result<Option<Value>> {
        let value = self
            .field
            .on_retrieve()
            .await?
            .ok_or_else(|| eyre!("Expected to get value from custom field"))?;
        Ok(Some(value))
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

        title.style.set(match selected {
            true => theme.specific.text.selected_subtitle,
            false => theme.specific.text.subtitle,
        });

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
    default_value: String,
    is_default: bool,
    field: Box<dyn Field<Value>>,
    menu: Option<Shared_widget<Menu<Option<Value>>>>,
}

impl<Value: Clone + Thread_safe> Optional_setting<Value> {
    pub fn new(
        default_value: impl Into<String>,
        is_default: bool,
        field: impl Field<Value> + 'static,
    ) -> Self {
        Self {
            default_value: default_value.into(),
            is_default,
            field: Box::new(field),
            menu: None,
        }
    }

    async fn get_menu(&mut self) -> Result<Shared_widget<Menu<Option<Value>>>> {
        if let Some(menu) = &self.menu {
            return Ok(menu.clone());
        }

        let default_item: Menu_item<Option<Value>> = Box::new(Default_leaf_value {
            label: self.default_value.clone(),
            value: PhantomData,
        });
        let custom_item: Menu_item<Option<Value>> = Box::new(Custom_leaf_value {
            field: self.field.clone(),
        });
        let items = vec![default_item, custom_item];
        let default_index = usize::from(!self.is_default);
        let menu = Menu::new(items, default_index).await?.into_shared();
        self.menu = Some(menu.clone());
        Ok(menu)
    }
}

#[async_trait]
impl<Value: Clone + Thread_safe> Widget_trait for Optional_setting<Value> {
    async fn layout(
        &mut self,
        Layout_input { slots, .. }: Layout_input<'_>,
    ) -> Result<Children> {
        let menu = self.get_menu().await?;
        Ok(vec![display!(menu)])
    }
}

#[async_trait]
impl<Value: Clone + Thread_safe> Retrieve_handler<Option<Value>> for Optional_setting<Value> {
    async fn on_retrieve(&mut self) -> Result<Option<Value>> {
        self.get_menu().await?.on_retrieve().await
    }
}
