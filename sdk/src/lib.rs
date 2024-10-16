// Caryatid framework module SDK - main library exports
pub mod message_bus;
pub mod in_memory_bus;
pub mod context;
pub mod module;

// Flattened re-exports
pub use caryatid_macros::module;
pub use self::message_bus::MessageBus;
pub use self::message_bus::MessageBusExt;
pub use self::in_memory_bus::InMemoryBus;
pub use self::context::Context;
pub use self::module::Module;
