use core::convert::TryFrom;
use core::str;

use nom::{
    bytes::complete::{tag, take, take_while_m_n},
    combinator::{map, map_opt, map_res, recognize, verify},
    error::{context, ParseError},
    multi::{length_data, many1},
    number::complete::{be_u16, be_u8},
    sequence::{pair, tuple},
    IResult,
};

use crate::packet::*;

impl FixedHeader {
    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Self, E> {
        map(
            tuple((
                map_res(
                    be_u8,
                    |b| -> Result<_, num_enum::TryFromPrimitiveError<Type>> {
                        let packet_type = Type::try_from((b >> 4) & 0x0F)?;
                        let packet_flags = b & 0x0F;

                        Ok((packet_type, packet_flags))
                    },
                ),
                variable_length,
            )),
            |((packet_type, packet_flags), remaining_length)| FixedHeader {
                packet_type,
                packet_flags,
                remaining_length,
            },
        )(input)
    }
}

const CONTINUATION_BIT: u8 = 0x80;

fn variable_length<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], usize, E> {
    context(
        "variable length",
        map(
            verify(
                recognize(pair(
                    take_while_m_n(0, 3, |b| (b & CONTINUATION_BIT) != 0),
                    verify(be_u8, |b| (b & CONTINUATION_BIT) == 0),
                )),
                |s: &[u8]| s.len() <= 4,
            ),
            |s: &[u8]| {
                s.iter().enumerate().fold(0, |value, (i, b)| {
                    value + (usize::from(*b & !CONTINUATION_BIT) << (7 * i))
                })
            },
        ),
    )(input)
}

/// Text fields in the Control Packets described later are encoded as UTF-8 strings.
fn utf8_str<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], &'a str, E> {
    context("utf8 string", map_res(length_data(be_u16), str::from_utf8))(input)
}

const CLIENT_ID_MIN_LEN: usize = 1;
const CLIENT_ID_MAX_LEN: usize = 23;
const CLIENT_ID_CHARS: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// The Client Identifier (ClientId) identifies the Client to the Server.
///
/// Each Client connecting to the Server has a unique ClientId.
/// The ClientId MUST be used by Clients and by Servers to identify state
/// that they hold relating to this MQTT Session between the Client and the Server [MQTT-3.1.3-2].
///
// The Server MUST allow ClientIds which are between 1 and 23 UTF-8 encoded bytes in length,
// and that contain only the characters [MQTT-3.1.3-5].
fn client_id<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], &'a str, E> {
    context(
        "client id",
        verify(utf8_str, |s: &str| -> bool {
            (s.is_empty() || (CLIENT_ID_MIN_LEN <= s.len() && s.len() <= CLIENT_ID_MAX_LEN))
                && s.bytes().all(|b| CLIENT_ID_CHARS.contains(&b))
        }),
    )(input)
}

/// The Topic Name identifies the information channel to which payload data is published.
///
/// The label attached to an Application Message which is matched against the Subscriptions known to the Server.
/// The Server sends a copy of the Application Message to each Client that has a matching Subscription.
fn topic_name<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], &'a str, E> {
    context(
        "topic name",
        verify(utf8_str, |s: &str| s.bytes().all(|c| c != b'#')),
    )(input)
}

/// An expression contained in a Subscription, to indicate an interest in one or more topics.
///
/// A Topic Filter can include wildcard characters.
fn topic_filter<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], &'a str, E> {
    context("topic filter", utf8_str)(input)
}

/// A Subscription comprises a Topic Filter and a maximum QoS.
/// A Subscription is associated with a single Session.
/// A Session can contain more than one Subscription.
/// Each Subscription within a session has a different Topic Filter.
fn subscription<'a, E: ParseError<&'a [u8]>>(
    input: &'a [u8],
) -> IResult<&'a [u8], (&'a str, QoS), E> {
    context(
        "subscription",
        tuple((topic_filter, context("QoS", map_res(be_u8, QoS::try_from)))),
    )(input)
}

fn packet_id<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], PacketId, E> {
    context("packet_id", be_u16)(input)
}

impl Packet<'_> {
    pub fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Packet<'a>, E> {
        let (input, fixed_header) = FixedHeader::parse(input)?;
        let (remaining, input) = take(fixed_header.remaining_length)(input)?;

        match fixed_header.packet_type {
            Type::CONNECT => map(Connect::parse, Packet::Connect)(input),
            Type::CONNACK => map(ConnectAck::parse, Packet::ConnectAck)(input),
            Type::PUBLISH => map(
                |i| {
                    Publish::parse(
                        PublishFlags::from_bits_truncate(fixed_header.packet_flags),
                        i,
                    )
                },
                Packet::Publish,
            )(input),
            Type::PUBACK => map(PublishAck::parse, Packet::PublishAck)(input),
            Type::PUBREC => map(PublishReceived::parse, Packet::PublishReceived)(input),
            Type::PUBREL => map(PublishRelease::parse, Packet::PublishRelease)(input),
            Type::PUBCOMP => map(PublishComplete::parse, Packet::PublishComplete)(input),
            Type::SUBSCRIBE => map(Subscribe::parse, Packet::Subscribe)(input),
            Type::SUBACK => map(SubscribeAck::parse, Packet::SubscribeAck)(input),
            Type::UNSUBSCRIBE => map(Unsubscribe::parse, Packet::Unsubscribe)(input),
            Type::UNSUBACK => map(UnsubscribeAck::parse, Packet::UnsubscribeAck)(input),
            Type::PINGREQ => Ok((remaining, Packet::Ping)),
            Type::PINGRESP => Ok((remaining, Packet::Pong)),
            Type::DISCONNECT => Ok((remaining, Packet::Disconnect)),
        }
    }
}

impl Connect<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Connect<'a>, E> {
        let (input, (_, _, flags, keep_alive)) = tuple((
            context("protocol name", tag(PROTOCOL_NAME)),
            context(
                "protocol level",
                verify(be_u8, |&level| level == PROTOCOL_LEVEL),
            ),
            context("flags", map_opt(be_u8, ConnectFlags::from_bits)),
            context("keepalive", be_u16),
        ))(input)?;
        let (input, client_id) = client_id(input)?;
        let (input, last_will) = if flags.contains(ConnectFlags::LAST_WILL) {
            let (input, (topic, message)) = tuple((
                context("will topic", utf8_str),
                context("will message", length_data(be_u16)),
            ))(input)?;

            (
                input,
                Some(LastWill {
                    qos: flags.qos(),
                    retain: flags.contains(ConnectFlags::WILL_RETAIN),
                    topic,
                    message,
                }),
            )
        } else {
            (input, None)
        };
        let (input, username) = if flags.contains(ConnectFlags::USERNAME) {
            context("username", map(utf8_str, Some))(input)?
        } else {
            (input, None)
        };
        let (input, password) = if flags.contains(ConnectFlags::PASSWORD) {
            context("password", map(length_data(be_u16), Some))(input)?
        } else {
            (input, None)
        };

        Ok((
            input,
            Connect {
                clean_session: flags.contains(ConnectFlags::CLEAN_SESSION),
                keep_alive,
                client_id,
                last_will,
                username,
                password,
            },
        ))
    }
}

impl ConnectAck {
    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Self, E> {
        map(
            tuple((
                context("flags", map_opt(be_u8, ConnectAckFlags::from_bits)),
                context("return code", map_res(be_u8, ConnectReturnCode::try_from)),
            )),
            |(flags, return_code)| ConnectAck {
                session_present: flags.contains(ConnectAckFlags::SESSION_PRESENT),
                return_code,
            },
        )(input)
    }
}

impl Publish<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        flags: PublishFlags,
        input: &'a [u8],
    ) -> IResult<&'a [u8], Publish<'a>, E> {
        let dup = flags.contains(PublishFlags::DUP);
        let qos = flags.qos();
        let retain = flags.contains(PublishFlags::RETAIN);
        let (input, topic) = topic_name(input)?;
        let (payload, packet_id) = if qos >= QoS::AtLeastOnce {
            map(packet_id, Some)(input)?
        } else {
            (input, None)
        };

        Ok((
            &[][..],
            Publish {
                dup,
                qos,
                retain,
                topic,
                packet_id,
                payload,
            },
        ))
    }
}

impl PublishAck {
    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Self, E> {
        map(packet_id, |packet_id| Self { packet_id })(input)
    }
}

impl PublishReceived {
    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Self, E> {
        map(packet_id, |packet_id| Self { packet_id })(input)
    }
}

impl PublishRelease {
    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Self, E> {
        map(packet_id, |packet_id| Self { packet_id })(input)
    }
}

impl PublishComplete {
    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Self, E> {
        map(packet_id, |packet_id| Self { packet_id })(input)
    }
}

impl Subscribe<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Subscribe<'a>, E> {
        map(
            tuple((packet_id, many1(subscription))),
            |(packet_id, subscriptions)| Subscribe {
                packet_id,
                subscriptions,
            },
        )(input)
    }
}

impl SubscribeAck {
    pub const FAILURE: u8 = 0x80;
    const QOS_MASK: u8 = 0x3;

    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Self, E> {
        map(
            tuple((
                packet_id,
                many1(context(
                    "return code",
                    map(be_u8, |b| {
                        if (b & Self::FAILURE) == 0 {
                            SubscribeReturnCode::Success(unsafe {
                                QoS::from_unchecked(b & Self::QOS_MASK)
                            })
                        } else {
                            SubscribeReturnCode::Failure
                        }
                    }),
                )),
            )),
            |(packet_id, status)| SubscribeAck { packet_id, status },
        )(input)
    }
}

impl Unsubscribe<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
    ) -> IResult<&'a [u8], Unsubscribe<'a>, E> {
        map(
            tuple((packet_id, many1(topic_filter))),
            |(packet_id, topic_filters)| Unsubscribe {
                packet_id,
                topic_filters,
            },
        )(input)
    }
}

impl UnsubscribeAck {
    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Self, E> {
        map(packet_id, |packet_id| Self { packet_id })(input)
    }
}

#[cfg(test)]
mod tests {
    use nom::error::{ErrorKind::*, VerboseError, VerboseErrorKind::*};

    use super::*;

    #[test]
    fn test_fixed_header() {
        assert_eq!(
            FixedHeader::parse::<()>(b"\x20\x7f"),
            Ok((
                &b""[..],
                FixedHeader {
                    packet_type: Type::CONNACK,
                    packet_flags: 0,
                    remaining_length: 127,
                },
            ))
        );

        assert_eq!(
            FixedHeader::parse::<()>(b"\x3C\x82\x7f"),
            Ok((
                &b""[..],
                FixedHeader {
                    packet_type: Type::PUBLISH,
                    packet_flags: 0x0C,
                    remaining_length: 16258,
                },
            ))
        );

        assert_eq!(
            FixedHeader::parse(b"\x20"),
            Err(nom::Err::Error((&b""[..], Eof))),
            "incomplete fixed header"
        );
    }

    #[test]
    fn test_variable_length() {
        macro_rules! assert_variable_length (
            ($bytes:expr, $res:expr) => {{
                assert_eq!(variable_length::<()>($bytes), Ok((&b""[..], $res)));
            }};

            ($bytes:expr, $res:expr, $rest:expr) => {{
                assert_eq!(variable_length::<()>($bytes), Ok((&$rest[..], $res)));
            }};
        );

        assert_variable_length!(b"\x7f\x7f", 127, b"\x7f");

        assert_eq!(
            variable_length(b"\xff\xff\xff"),
            Err(nom::Err::Error((&b""[..], Eof))),
            "incomplete variable length"
        );
        assert_eq!(
            variable_length(b"\xff\xff\xff\xff\xff\xff"),
            Err(nom::Err::Error((&b"\xff\xff\xff"[..], Verify))),
            "too long variable length"
        );

        assert_variable_length!(b"\x00", 0);
        assert_variable_length!(b"\x7f", 127);
        assert_variable_length!(b"\x80\x01", 128);
        assert_variable_length!(b"\xff\x7f", 16383);
        assert_variable_length!(b"\x80\x80\x01", 16384);
        assert_variable_length!(b"\xff\xff\x7f", 2097151);
        assert_variable_length!(b"\x80\x80\x80\x01", 2097152);
        assert_variable_length!(b"\xff\xff\xff\x7f", 268435455);
    }

    #[test]
    fn test_connect() {
        assert_eq!(
            Connect::parse::<()>(
                b"\x00\x04MQTT\x04\xC0\x00\x3C\x00\x0512345\x00\x04user\x00\x04pass",
            ),
            Ok((
                &b""[..],
                Connect {
                    clean_session: false,
                    keep_alive: 60,
                    client_id: "12345",
                    last_will: None,
                    username: Some("user"),
                    password: Some(b"pass"),
                },
            ))
        );

        assert_eq!(
            Connect::parse::<()>(
                b"\x00\x04MQTT\x04\x14\x00\x3C\x00\x0512345\x00\x05topic\x00\x07message",
            ),
            Ok((
                &b""[..],
                Connect {
                    clean_session: false,
                    keep_alive: 60,
                    client_id: "12345",
                    last_will: Some(LastWill {
                        qos: QoS::ExactlyOnce,
                        retain: false,
                        topic: "topic",
                        message: b"message",
                    }),
                    username: None,
                    password: None,
                },
            ))
        );

        assert_eq!(
            Connect::parse(b"\x00\x02MQ"),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&b"\x00\x02MQ"[..], Nom(Tag)),
                    (&b"\x00\x02MQ"[..], Context("protocol name"))
                ],
            })),
            "incomplete protocol name"
        );
        assert_eq!(
            Connect::parse(b"\x00\x04MQAA"),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&b"\x00\x04MQAA"[..], Nom(Tag)),
                    (&b"\x00\x04MQAA"[..], Context("protocol name"))
                ],
            })),
            "invalid protocol name"
        );
        assert_eq!(
            Connect::parse(b"\x00\x04MQTT\x03"),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&b"\x03"[..], Nom(Verify)),
                    (&b"\x03"[..], Context("protocol level"))
                ],
            })),
            "invalid protocol level"
        );
        assert_eq!(
            Connect::parse(b"\x00\x04MQTT\x04\xff"),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&b"\xff"[..], Nom(MapOpt)),
                    (&b"\xff"[..], Context("flags"))
                ],
            })),
            "invalid packet flags"
        );
    }

    #[test]
    fn test_connect_ack() {
        assert_eq!(
            ConnectAck::parse::<()>(b"\x01\x04"),
            Ok((
                &b""[..],
                ConnectAck {
                    session_present: true,
                    return_code: ConnectReturnCode::BadUserNameOrPassword,
                }
            ))
        );

        assert_eq!(
            ConnectAck::parse(b"\x03\x04"),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&b"\x03\x04"[..], Nom(MapOpt)),
                    (&b"\x03\x04"[..], Context("flags")),
                ]
            })),
            "invalid flags"
        );
    }

    #[test]
    fn test_disconnect() {
        assert_eq!(
            Packet::parse::<()>(b"\xe0\x00"),
            Ok((&b""[..], Packet::Disconnect))
        );
    }

    #[test]
    fn test_publish() {
        assert_eq!(
            Publish::parse::<()>(QoS::AtLeastOnce.into(), b"\x00\x05topic\x12\x34hello"),
            Ok((
                &b""[..],
                Publish {
                    dup: false,
                    qos: QoS::AtLeastOnce,
                    retain: false,
                    topic: "topic",
                    packet_id: Some(0x1234),
                    payload: b"hello",
                }
            ))
        );

        assert_eq!(
            Packet::parse::<()>(b"\x3d\x0D\x00\x05topic\x43\x21data"),
            Ok((
                &b""[..],
                Packet::Publish(Publish {
                    dup: true,
                    retain: true,
                    qos: QoS::ExactlyOnce,
                    topic: "topic",
                    packet_id: Some(0x4321),
                    payload: b"data",
                }),
            ))
        );
        assert_eq!(
            Packet::parse::<()>(b"\x30\x0b\x00\x05topicdata"),
            Ok((
                &b""[..],
                Packet::Publish(Publish {
                    dup: false,
                    retain: false,
                    qos: QoS::AtMostOnce,
                    topic: "topic",
                    packet_id: None,
                    payload: b"data",
                }),
            ))
        );

        assert_eq!(
            Packet::parse::<()>(b"\x40\x02\x43\x21"),
            Ok((
                &b""[..],
                Packet::PublishAck(PublishAck { packet_id: 0x4321 })
            ))
        );
        assert_eq!(
            Packet::parse::<()>(b"\x50\x02\x43\x21"),
            Ok((
                &b""[..],
                Packet::PublishReceived(PublishReceived { packet_id: 0x4321 })
            ))
        );
        assert_eq!(
            Packet::parse::<()>(b"\x62\x02\x43\x21"),
            Ok((
                &b""[..],
                Packet::PublishRelease(PublishRelease { packet_id: 0x4321 })
            ))
        );
        assert_eq!(
            Packet::parse::<()>(b"\x70\x02\x43\x21"),
            Ok((
                &b""[..],
                Packet::PublishComplete(PublishComplete { packet_id: 0x4321 })
            ))
        );
    }

    #[test]
    fn test_subscribe() {
        assert_eq!(
            Subscribe::parse::<()>(b"\x12\x34\x00\x04test\x01\x00\x06filter\x02"),
            Ok((
                &b""[..],
                Subscribe {
                    packet_id: 0x1234,
                    subscriptions: vec![("test", QoS::AtLeastOnce), ("filter", QoS::ExactlyOnce)],
                }
            ))
        );
        assert_eq!(
            Packet::parse::<()>(b"\x82\x12\x12\x34\x00\x04test\x01\x00\x06filter\x02"),
            Ok((
                &b""[..],
                Packet::Subscribe(Subscribe {
                    packet_id: 0x1234,
                    subscriptions: vec![("test", QoS::AtLeastOnce), ("filter", QoS::ExactlyOnce)],
                })
            ))
        );

        assert_eq!(
            SubscribeAck::parse::<()>(b"\x12\x34\x01\x80\x02"),
            Ok((
                &b""[..],
                SubscribeAck {
                    packet_id: 0x1234,
                    status: vec![
                        SubscribeReturnCode::Success(QoS::AtLeastOnce),
                        SubscribeReturnCode::Failure,
                        SubscribeReturnCode::Success(QoS::ExactlyOnce),
                    ],
                }
            ))
        );

        assert_eq!(
            Packet::parse::<()>(b"\x90\x05\x12\x34\x01\x80\x02"),
            Ok((
                &b""[..],
                Packet::SubscribeAck(SubscribeAck {
                    packet_id: 0x1234,
                    status: vec![
                        SubscribeReturnCode::Success(QoS::AtLeastOnce),
                        SubscribeReturnCode::Failure,
                        SubscribeReturnCode::Success(QoS::ExactlyOnce),
                    ],
                })
            ))
        );

        assert_eq!(
            Unsubscribe::parse::<()>(b"\x12\x34\x00\x04test\x00\x06filter"),
            Ok((
                &b""[..],
                Unsubscribe {
                    packet_id: 0x1234,
                    topic_filters: vec!["test", "filter"],
                }
            ))
        );
        assert_eq!(
            Packet::parse::<()>(b"\xa2\x10\x12\x34\x00\x04test\x00\x06filter"),
            Ok((
                &b""[..],
                Packet::Unsubscribe(Unsubscribe {
                    packet_id: 0x1234,
                    topic_filters: vec!["test", "filter"],
                })
            ))
        );

        assert_eq!(
            Packet::parse::<()>(b"\xb0\x02\x43\x21"),
            Ok((
                &b""[..],
                Packet::UnsubscribeAck(UnsubscribeAck { packet_id: 0x4321 })
            ))
        );

        assert_eq!(
            Packet::parse(b"\x82\x02\x42\x42"),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&[][..], Nom(Eof)),
                    (&[][..], Context("utf8 string")),
                    (&[][..], Context("topic filter")),
                    (&[][..], Context("subscription")),
                    (&[][..], Nom(Many1))
                ]
            })),
            "subscribe without subscription topics"
        );

        assert_eq!(
            Packet::parse(b"\x82\x04\x42\x42\x00\x00"),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&[][..], Nom(Eof)),
                    (&[][..], Context("QoS")),
                    (&[0, 0][..], Context("subscription")),
                    (&[0, 0][..], Nom(Many1))
                ]
            })),
            "malformed/malicious subscribe packets: no QoS for topic filter"
        );

        assert_eq!(
            Packet::parse(b"\x82\x03\x42\x42\x00"),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&[0][..], Nom(Eof)),
                    (&[0][..], Context("utf8 string")),
                    (&[0][..], Context("topic filter")),
                    (&[0][..], Context("subscription")),
                    (&[0][..], Nom(Many1))
                ]
            })),
            "truncated string length prefix"
        );

        assert_eq!(
            Packet::parse(b"\xa2\x02\x42\x42"),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&[][..], Nom(Eof)),
                    (&[][..], Context("utf8 string")),
                    (&[][..], Context("topic filter")),
                    (&[][..], Nom(Many1))
                ]
            })),
            "unsubscribe without subscription topics"
        );

        assert_eq!(
            Packet::parse(b"\xa2\x03\x42\x42\x00"),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&[0][..], Nom(Eof)),
                    (&[0][..], Context("utf8 string")),
                    (&[0][..], Context("topic filter")),
                    (&[0][..], Nom(Many1))
                ]
            })),
            "malformed/malicious unsubscribe packets: truncated string length prefix"
        );
    }

    #[test]
    fn test_ping_pong() {
        assert_eq!(
            Packet::parse::<()>(b"\xc0\x00"),
            Ok((&b""[..], Packet::Ping))
        );
        assert_eq!(
            Packet::parse::<()>(b"\xd0\x00"),
            Ok((&b""[..], Packet::Pong))
        );
    }
}
