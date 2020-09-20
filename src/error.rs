use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErrorKind {
    #[error("Part of the token authentication process failed: `{0}`")]
    TokenRetrievalError(String),
    #[error("Part of the ID Retrieval process failed: `{0}`")]
    IDRetrievalError(String),
    #[error("Server did not respond with 2xx: `{0}`")]
    ServerError(String),
    #[error("GenericError: `{0}`")]
    GenericError(String),
}

impl From<yup_oauth2::error::Error> for ErrorKind {
    fn from(e: yup_oauth2::error::Error) -> Self {
        ErrorKind::TokenRetrievalError(format!("OAuth Error: {}", e.to_string()))
    }
}

impl From<std::io::Error> for ErrorKind {
    fn from(e: std::io::Error) -> Self {
        ErrorKind::GenericError(format!("IO Error: {}", e.to_string()))
    }
}
