pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Serenity Error: {0}")]
    SerenityError(#[from] serenity::Error),

    #[error("Page {0} not found")]
    PageNotFound(usize),

    #[error("Serenity Rich Interaction is not fully initialized")]
    Uninitialized,

    #[error("the cache is not available, therefore some required data is missing")]
    NoCache,

    #[error("{0}")]
    Msg(String),
}
