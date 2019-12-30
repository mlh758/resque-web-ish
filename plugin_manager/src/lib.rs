use libloading::{Library, Symbol};
use std::ffi::OsStr;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

mod plugin;
pub use plugin::{Action, Plugin};

/// Handles plugins for the root resque web application
/// Plugins are dynamically loaded at application start and must satisfy
/// the `Plugin` trait defined in this crate. Plugins will be called in the order
/// they are added to the manager.
pub struct PluginManager {
    plugins: Vec<Box<dyn Plugin>>,
    loaded_libraries: Vec<Library>,
}

type PluginCreate = unsafe fn() -> *mut dyn Plugin;

impl PluginManager {
    pub fn new() -> PluginManager {
        PluginManager {
            plugins: Vec::new(),
            loaded_libraries: Vec::new(),
        }
    }

    /// Loads the file at the given path as a plugin (so, dll, dylib).
    ///
    /// # Panics
    ///
    /// This function will panic if the pointer from _plugin_create points to invalid memory.
    /// Use the macro defined in this crate to define _plugin_create function for your library to help
    /// mitigate that risk.
    pub unsafe fn load_plugin(&mut self, path: PathBuf) -> libloading::Result<()> {
        let lib = Library::new(OsStr::new(&path))?;
        self.loaded_libraries.push(lib);
        let lib = self.loaded_libraries.last().unwrap();

        let constructor: Symbol<PluginCreate> = lib.get(b"_plugin_create")?;
        let boxed_raw = constructor();
        if boxed_raw.is_null() {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "null pointer from constructor",
            ));
        }

        let plugin = Box::from_raw(boxed_raw);
        plugin.on_plugin_load();
        self.plugins.push(plugin);

        Ok(())
    }

    /// Attempts to load every file in the given directory as a dynamic library satisfying
    /// the `Plugin` trait. It naively iterates over the files and loads them, ignoring nested
    /// directories.
    ///
    /// # Panics
    ///
    /// The call to load_plugin can panic, see that function's documentation
    pub fn load_directory(&mut self, directory: &str) -> libloading::Result<()> {
        if directory.is_empty() {
            return Ok(());
        }
        let plugin_path = Path::new(directory);
        for entry in plugin_path.read_dir()? {
            if let Ok(entry) = entry {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        unsafe { self.load_plugin(entry.path())? }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn pre_action(&self, action: Action) {
        for plugin in &self.plugins {
            plugin.before_action(&action);
        }
    }

    pub fn post_action(&self, action: Action) {
        for plugin in &self.plugins {
            plugin.after_action(&action);
        }
    }

    pub fn unload(&mut self) {
        for plugin in self.plugins.drain(..) {
            plugin.on_plugin_unload();
        }
        for lib in self.loaded_libraries.drain(..) {
            drop(lib);
        }
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        self.unload();
    }
}

/// This macro was taken from https://michael-f-bryan.github.io/rust-ffi-guide/dynamic_loading.html
/// Use this to export your plugin type so that it can be loaded by this library. It expects a type
/// satisfying the trait and a constructor for that type.
/// `declare_plugin!(SomePlugin, SomePlugin::new)`
#[macro_export]
macro_rules! declare_plugin {
    ($plugin_type:ty, $constructor:path) => {
        #[no_mangle]
        pub extern "C" fn _plugin_create() -> *mut $crate::Plugin {
            // make sure the constructor is the correct type.
            let constructor: fn() -> $plugin_type = $constructor;

            let object = constructor();
            let boxed: Box<$crate::Plugin> = Box::new(object);
            Box::into_raw(boxed)
        }
    };
}
