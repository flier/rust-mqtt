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
    }
}
