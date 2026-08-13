#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("Failed to build proble. Bad input: {0}")]
    BadProblemInput(String),
}

pub type Result<T> = std::result::Result<T, Error>;
