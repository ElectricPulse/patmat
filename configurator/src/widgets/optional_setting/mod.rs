use async_trait::async_trait;
use color_eyre::eyre::Result;
use derive_where::derive_where;
use std::{marker::PhantomData, sync::Arc};
use vizual::geometry::Direction;
use vizual::{
    Signal,
    component::Children,
    handlers::RetrieveHandler,
    state::{Constant, ReadGuard, State, StateTrait},
    sync::ThreadSafe,
    widget::{
        LayoutInput, WidgetTrait,
        custom_widget::CustomWidgetTrait,
        widgets::{
            layout::axis::Axis,
            menu::{Menu, MenuItem},
            positioning::anchor::Anchor,
            text::Text,
        },
    },
};
use vizual_macros::display;

use crate::Field;

#[derive(Clone)]
struct SomeState<Value: ThreadSafe>(State<Value>);

#[async_trait]
impl<Value: ThreadSafe + Clone> StateTrait for SomeState<Value> {
    type Output = Option<Value>;

    async fn read(&self) -> Result<ReadGuard<Self::Output>> {
        let guard = self.0.read().await?;
        Ok(ReadGuard::new(Arc::new(Some((*guard).clone()))))
    }

    async fn affect(&self, signal: Signal) -> Result<ReadGuard<Self::Output>> {
        let guard = self.0.affect(signal).await?;
        Ok(ReadGuard::new(Arc::new(Some((*guard).clone()))))
    }
}

#[derive_where(Clone)]
struct DefaultLeafValue<Value: ThreadSafe> {
    label: String,
    value: PhantomData<Value>,
}

#[async_trait]
impl<Value: ThreadSafe> RetrieveHandler<Option<Value>> for DefaultLeafValue<Value> {
    async fn on_retrieve(&mut self) -> Result<State<Option<Value>>> {
        Ok(Constant::from(None).into())
    }
}

#[async_trait]
impl<Value: ThreadSafe> CustomWidgetTrait for DefaultLeafValue<Value> {
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
        let theme = theme.affect(relayout).await?;
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
struct CustomLeafValue<Value: ThreadSafe> {
    field: Box<dyn Field<Value>>,
}

#[async_trait]
impl<Value: ThreadSafe + Clone> RetrieveHandler<Option<Value>> for CustomLeafValue<Value> {
    async fn on_retrieve(&mut self) -> Result<State<Option<Value>>> {
        let inner_state = self.field.on_retrieve().await?;
        Ok(Box::new(SomeState(inner_state)))
    }
}

#[async_trait]
impl<Value: ThreadSafe> CustomWidgetTrait for CustomLeafValue<Value> {
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
        let theme = theme.affect(relayout).await?;
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
pub struct OptionalSetting<Value: Clone + ThreadSafe> {
    menu: Menu<Option<Value>>,
}

impl<Value: Clone + ThreadSafe> OptionalSetting<Value> {
    pub async fn new(
        default_value: impl Into<String>,
        is_default: bool,
        field: impl Field<Value> + 'static,
    ) -> Result<Self> {
        let default_item: MenuItem<Option<Value>> = Box::new(DefaultLeafValue {
            label: default_value.into(),
            value: PhantomData,
        });
        let custom_item: MenuItem<Option<Value>> = Box::new(CustomLeafValue {
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
impl<Value: Clone + ThreadSafe> WidgetTrait for OptionalSetting<Value> {
    async fn layout(&mut self, LayoutInput { slots, .. }: LayoutInput<'_>) -> Result<Children> {
        Ok(vec![display!(self.menu.clone())])
    }
}

#[async_trait]
impl<Value: Clone + ThreadSafe> RetrieveHandler<Option<Value>> for OptionalSetting<Value> {
    async fn on_retrieve(&mut self) -> Result<State<Option<Value>>> {
        self.menu.on_retrieve().await
    }
}

#[cfg(test)]
mod tests;
