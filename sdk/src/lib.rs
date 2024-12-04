// Caryatid framework module SDK - main library exports
pub mod message_bus;
pub mod context;
pub mod module;
pub mod module_registry;
pub mod config;
pub mod messages;
pub mod match_topic;
pub mod mock_bus;
pub mod correlation_bus;

// Flattened re-exports
pub use caryatid_macros::module;
pub use self::message_bus::MessageBounds;
pub use self::message_bus::MessageBus;
pub use self::message_bus::MessageBusExt;
pub use self::context::Context;
pub use self::module::Module;
pub use self::module_registry::ModuleRegistry;
