// Caryatid framework module SDK - main library exports
pub mod config;
pub mod context;
pub mod constants;
pub mod match_topic;
pub mod message_bus;
pub mod mock_bus;
pub mod module;
pub mod module_registry;

// Flattened re-exports
pub use self::context::Context;
pub use self::message_bus::MessageBounds;
pub use self::message_bus::MessageBus;
pub use self::message_bus::Subscription;
pub use self::message_bus::SubscriptionBounds;
pub use self::module::Module;
pub use self::module_registry::ModuleRegistry;
pub use async_trait::async_trait;
pub use caryatid_macros::module;
