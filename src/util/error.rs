pub use failure::Error;

/// A result using Fail
pub type Res<T> = Result<T, failure::Error>;
