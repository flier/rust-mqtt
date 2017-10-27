use std::sync::{PoisonError, TryLockError};

error_chain! {
    foreign_links {
        Fmt(::std::fmt::Error);
        Io(::std::io::Error);
    }

    errors {
        ConnectFailed(code: ::core::ConnectReturnCode) {
            description("connect failed")
            display("connect failed, {:?}", code)
        }
        ProtocolViolation
        ConnectionClosed
        InvalidTopic(topic: String) {
            description("invalid topic")
            display("invalid topic, {}", topic)
        }
        InvalidPacketId
        UnexpectedState
        BadUserNameOrPassword
        LockError(reason: String) {
            description("lock failed")
            display("lock failed, {}", reason)
        }
    }
}

impl<T> From<PoisonError<T>> for Error {
    fn from(err: PoisonError<T>) -> Self {
        ErrorKind::LockError(err.to_string()).into()
    }
}

impl<T> From<TryLockError<T>> for Error {
    fn from(err: TryLockError<T>) -> Self {
        ErrorKind::LockError(err.to_string()).into()
    }
}
