pub use failure::Error;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "out of memory")]
    OutOfMemory,

    #[fail(display = "invalid state")]
    InvalidState,

    #[fail(display = "invalid packet")]
    InvalidPacket,

    #[fail(display = "invalid topic")]
    InvalidTopic,

    #[fail(display = "spwan error")]
    SpawnError,
}

impl<T> From<::rotor::SpawnError<T>> for ErrorKind {
    fn from(err: ::rotor::SpawnError<T>) -> Self {
        match err {
            ::rotor::SpawnError::NoSlabSpace(_) => ErrorKind::OutOfMemory.into(),
            ::rotor::SpawnError::UserError(_) => ErrorKind::SpawnError.into(),
        }
    }
}
