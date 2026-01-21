use std::fmt;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Kube(kube::Error),
    Hyper(hyper::Error),
    Json(serde_json::Error),
    NotFound(String),
    BadRequest(String),
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::Kube(e) => write!(f, "Kubernetes error: {}", e),
            Error::Hyper(e) => write!(f, "Hyper error: {}", e),
            Error::Json(e) => write!(f, "JSON error: {}", e),
            Error::NotFound(msg) => write!(f, "Not found: {}", msg),
            Error::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            Error::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<kube::Error> for Error {
    fn from(e: kube::Error) -> Self {
        Error::Kube(e)
    }
}

impl From<hyper::Error> for Error {
    fn from(e: hyper::Error) -> Self {
        Error::Hyper(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Json(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

