pub mod config;
pub mod lifecycle;
pub mod manifest;
pub mod registry;
pub mod sandbox;
pub mod traits;

pub use config::PluginConfigSchema;
pub use lifecycle::{PluginInstance, PluginState};
pub use manifest::PluginManifest;
pub use registry::PluginRegistry;
pub use traits::{
    ExporterPlugin, GeneratorPlugin, Plugin, PluginContext, PluginType, PostProcessPlugin,
    StylePlugin,
};
