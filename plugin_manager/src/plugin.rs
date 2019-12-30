use std::any::Any;

/// Provides information about the action your plugin is being called for
pub enum Action {
  DeleteQueue(String),
  RetryAll,
}

/// Defines an interface for plugins to adhere to.
pub trait Plugin: Any + Send + Sync {
  fn name(&self) -> &'static str;
  fn on_plugin_load(&self) {}
  fn on_plugin_unload(&self) {}
  fn before_action(&self, _action: &Action) {}
  fn after_action(&self, _action: &Action) {}
}
