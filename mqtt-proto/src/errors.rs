use std::error::Error as _;

pub use failure::Error;

use crate::core::ConnectReturnCode;

pub type Result<T> = ::std::result::Result<T, Error>;

#[derive(Debug, Fail)]
pub enum ErrorKind {
    #[fail(display = "connect failed, {:?}", _0)]
    ConnectFailed(ConnectReturnCode),

    #[fail(display = "connection closed")]
    ConnectionClosed,

    #[fail(display = "protocol violation")]
    ProtocolViolation,

    #[fail(display = "invalid packet id")]
    InvalidPacketId,

    #[fail(display = "invalid topic name, {}", _0)]
    InvalidTopic(String),

    #[fail(display = "bad user name or password")]
    BadUserNameOrPassword,

    #[fail(display = "unexpected protocol state")]
    UnexpectedState,

    #[fail(display = "fail to lock, {}", _0)]
    LockFailed(String),

    #[fail(display = "fail to send to queue, {}", _0)]
    MpscSendError(String),
}

impl<T> From<std::sync::PoisonError<T>> for ErrorKind {
    fn from(err: std::sync::PoisonError<T>) -> Self {
        ErrorKind::LockFailed(err.description().to_owned())
    }
}

impl<T> From<std::sync::TryLockError<T>> for ErrorKind {
    fn from(err: std::sync::TryLockError<T>) -> Self {
        ErrorKind::LockFailed(err.description().to_owned())
    }
}

impl<T> From<futures::sync::mpsc::SendError<T>> for ErrorKind {
    fn from(err: futures::sync::mpsc::SendError<T>) -> Self {
        ErrorKind::MpscSendError(format!("{:?}", err))
    }
}
