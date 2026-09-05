use super::*;

#[tokio::test]
async fn default_returns_none() -> Result<()> {
    let submenu = Menu::text(vec!["One".into(), "Two".into()], 0).await?;
    let mut setting = OptionalSetting::new("DefaultOption", true, submenu).await?;

    let state = setting.on_retrieve().await?;
    let value = state.read().await?;
    assert_eq!(*value, None);
    Ok(())
}

#[tokio::test]
async fn custom_returns_selected_submenu_value() -> Result<()> {
    let submenu = Menu::text(vec!["One".into(), "Two".into()], 0).await?;
    let mut setting = OptionalSetting::new("DefaultOption", false, submenu).await?;

    let state = setting.on_retrieve().await?;
    let value = state.read().await?;
    assert_eq!(*value, Some("One".to_string()));
    Ok(())
}

#[tokio::test]
async fn changing_submenu_updates_optional_setting_state() -> Result<()> {
    let mut submenu = Menu::text(vec!["One".into(), "Two".into()], 0).await?;
    let mut setting = OptionalSetting::new("DefaultOption", false, submenu.clone()).await?;

    let state = setting.on_retrieve().await?;
    assert_eq!(*state.read().await?, Some("One".to_string()));

    submenu.set_index(1).await?;
    assert_eq!(*state.read().await?, Some("Two".to_string()));
    Ok(())
}

#[tokio::test]
async fn switching_between_default_and_custom() -> Result<()> {
    let submenu = Menu::text(vec!["One".into(), "Two".into()], 1).await?;
    let mut setting = OptionalSetting::new("DefaultOption", true, submenu).await?;

    let state = setting.on_retrieve().await?;
    assert_eq!(*state.read().await?, None);

    setting.set_is_default(false).await?;
    assert_eq!(*state.read().await?, Some("Two".to_string()));

    setting.set_is_default(true).await?;
    assert_eq!(*state.read().await?, None);
    Ok(())
}
