pub mod interfaces;
pub use interfaces::{IScreenCastProxy, IRequestProxy};

pub mod screencast_runner;
pub use screencast_runner::ScreenCastRunnerClient;

pub mod types;