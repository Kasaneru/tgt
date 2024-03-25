use {
    crate::{
        app_error::AppError,
        components::{
            component::Component, core_window::CoreWindow,
            status_bar::StatusBar, title_bar::TitleBar, SMALL_AREA_HEIGHT,
            SMALL_AREA_WIDTH,
        },
        configs::custom::{app_custom::AppConfig, keymap_custom::KeymapConfig},
        enums::{action::Action, component_name::ComponentName, event::Event},
    },
    ratatui::layout::{Constraint, Direction, Layout, Rect},
    std::{collections::HashMap, io},
    tokio::sync::mpsc::UnboundedSender,
};

/// `Tui` is a struct that represents the main user interface for the
/// application. It is responsible for managing the layout and rendering of all
/// the components. It also handles the distribution of events and actions to
/// the appropriate components.
pub struct Tui {
    /// An optional unbounded sender that can send actions to be processed.
    action_tx: Option<UnboundedSender<Action>>,
    /// A hashmap of components that make up the user interface.
    components: HashMap<ComponentName, Box<dyn Component>>,
    /// The application configuration.
    app_config: AppConfig,
    #[allow(dead_code)]
    /// The keymap configuration.
    keymap_config: KeymapConfig,
    /// The name of the component that currently has focus. It is an optional
    /// value because no component may have focus. The focus is a component
    /// inside the `CoreWindow`.
    focused: Option<ComponentName>,
}
/// Implement the `Default` trait for the `Tui` struct.
impl Default for Tui {
    fn default() -> Self {
        Self::new(AppConfig::default(), KeymapConfig::default())
    }
}
/// Implement the `Tui` struct.
impl Tui {
    /// Create a new instance of the `Tui` struct.
    ///
    /// # Arguments
    /// * `app_config` - The application configuration.
    /// * `keymap_config` - The keymap configuration.
    ///
    /// # Returns
    /// * `Self` - The new instance of the `Tui` struct.
    pub fn new(app_config: AppConfig, keymap_config: KeymapConfig) -> Self {
        let components_iter: Vec<(ComponentName, Box<dyn Component>)> = vec![
            (
                ComponentName::TitleBar,
                TitleBar::new().with_name("Tgt").new_boxed(),
            ),
            (
                ComponentName::CoreWindow,
                CoreWindow::new().with_name("CoreWindow").new_boxed(),
            ),
            (
                ComponentName::StatusBar,
                StatusBar::new().with_name("Status Bar").new_boxed(),
            ),
        ];

        let action_tx = None;
        let focused = None;
        let components: HashMap<ComponentName, Box<dyn Component>> =
            components_iter.into_iter().collect();

        Tui {
            action_tx,
            components,
            keymap_config,
            focused,
            app_config,
        }
    }
    /// Register an action handler that can send actions for processing if
    /// necessary.
    ///
    /// # Arguments
    ///
    /// * `tx` - An unbounded sender that can send actions.
    ///
    /// # Returns
    ///
    /// * `Result<()>` - An Ok result or an error.
    pub fn register_action_handler(
        &mut self,
        tx: UnboundedSender<Action>,
    ) -> Result<(), AppError> {
        self.action_tx = Some(tx.clone());
        self.components.iter_mut().try_for_each(|(_, component)| {
            component.register_action_handler(tx.clone())
        })?;
        Ok(())
    }
    /// Handle incoming events and produce actions if necessary.
    ///
    /// # Arguments
    ///
    /// * `event` - An optional event to be processed.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    pub fn handle_events(
        &mut self,
        event: Option<Event>,
    ) -> Result<Option<Action>, AppError> {
        self.components
            .get_mut(&ComponentName::CoreWindow)
            .unwrap()
            .handle_events(event.clone())
    }
    /// Update the state of the component based on a received action.
    ///
    /// # Arguments
    ///
    /// * `action` - An action that may modify the state of the component.
    ///
    /// # Returns
    ///
    /// * `Result<Option<Action>>` - An action to be processed or none.
    pub fn update(&mut self, action: Action) -> io::Result<Option<Action>> {
        match action {
            Action::FocusComponent(component_name) => {
                self.focused = Some(component_name);
            }
            Action::UnfocusComponent => {
                self.focused = None;
            }
            _ => {}
        }

        // We can not send the action only to the `CoreWindow` component because
        // the `StatusBar` component needs to know the area to render the size.
        self.components
            .iter_mut()
            .try_fold(None, |acc, (_, component)| {
                match component.update(action.clone()) {
                    Ok(Some(action)) => Ok(Some(action)),
                    Ok(None) => Ok(acc),
                    Err(e) => Err(e),
                }
            })
    }
    /// Render the user interface to the screen.
    ///
    /// # Arguments
    /// * `frame` - A mutable reference to the frame to be rendered.
    /// * `area` - A rectangular area to render the user interface within.
    ///
    /// # Returns
    /// * `Result<()>` - An Ok result or an error.
    pub fn draw(
        &mut self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
    ) -> Result<(), AppError> {
        self.components
            .get_mut(&ComponentName::StatusBar)
            .unwrap()
            .update(Action::UpdateArea(area))?;

        self.components
            .get_mut(&ComponentName::CoreWindow)
            .unwrap()
            .with_small_area(area.width < SMALL_AREA_WIDTH);

        let main_layout = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(if self.app_config.show_title_bar {
                    if area.height > SMALL_AREA_HEIGHT + 5 {
                        3
                    } else {
                        0
                    }
                } else {
                    0
                }),
                Constraint::Min(SMALL_AREA_HEIGHT),
                Constraint::Length(if self.app_config.show_status_bar {
                    if area.height > SMALL_AREA_HEIGHT + 5 {
                        3
                    } else {
                        0
                    }
                } else {
                    0
                }),
            ],
        )
        .split(area);

        self.components
            .get_mut(&ComponentName::TitleBar)
            .unwrap_or_else(|| {
                tracing::error!(
                    "Failed to get component: {}",
                    ComponentName::TitleBar
                );
                panic!("Failed to get component: {}", ComponentName::TitleBar)
            })
            .draw(frame, main_layout[0])?;

        self.components
            .get_mut(&ComponentName::CoreWindow)
            .unwrap_or_else(|| {
                tracing::error!(
                    "Failed to get component: {}",
                    ComponentName::CoreWindow
                );
                panic!("Failed to get component: {}", ComponentName::CoreWindow)
            })
            .draw(frame, main_layout[1])?;

        self.components
            .get_mut(&ComponentName::StatusBar)
            .unwrap_or_else(|| {
                tracing::error!(
                    "Failed to get component: {}",
                    ComponentName::StatusBar
                );
                panic!("Failed to get component: {}", ComponentName::StatusBar)
            })
            .draw(frame, main_layout[2])?;

        Ok(())
    }
}
