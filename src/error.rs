use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// Bad arguments, missing input (exit 1)
    Usage(String),
    /// File not found, corrupt image, invalid colour (exit 2)
    Input(String),
    /// Operation failed (exit 3)
    Processing(String),
}

impl Error {
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::Usage(_) => 1,
            Error::Input(_) => 2,
            Error::Processing(_) => 3,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Usage(msg) | Error::Input(msg) | Error::Processing(msg) => {
                write!(f, "{msg}")
            }
        }
    }
}

impl std::error::Error for Error {}
