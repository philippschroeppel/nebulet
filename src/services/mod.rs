pub mod processor;

// Make docker module private to services, only accessible by processor
mod docker;

pub use processor::*;
