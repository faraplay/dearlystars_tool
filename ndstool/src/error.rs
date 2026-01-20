use std::error::Error;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub(crate) struct NdsError {
    pub(crate) message: String,
}

impl std::fmt::Display for NdsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for NdsError {}
