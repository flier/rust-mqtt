use core::convert::TryFrom;
use core::str;
use core::time::Duration;
use core::u32;

use nom::{
    bytes::complete::{tag, take, take_while_m_n},
    combinator::{all_consuming, cond, map, map_opt, map_res, opt, recognize, rest, verify},
    error::{context, ParseError},
    multi::{length_data, many1},
    number::complete::{be_u16, be_u32, be_u8},
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
                varint,
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

fn varint<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], usize, E> {
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

/// Binary Data is represented by a Two Byte Integer length which indicates the number of data bytes,
/// followed by that number of bytes. Thus, the length of Binary Data is limited to the range of 0 to 65,535 Bytes.
fn binary_data<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], &'a [u8], E> {
    context("binary data", length_data(be_u16))(input)
}

/// Text fields in the Control Packets described later are encoded as UTF-8 strings.
fn utf8_str<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], &'a str, E> {
    context("utf8 string", map_res(length_data(be_u16), str::from_utf8))(input)
}

/// A UTF-8 String Pair consists of two UTF-8 Encoded Strings. This data type is used to hold name-value pairs.
/// The first string serves as the name, and the second string contains the value.
fn utf8_str_pair<'a, E: ParseError<&'a [u8]>>(
    input: &'a [u8],
) -> IResult<&'a [u8], (&'a str, &'a str), E> {
    context(
        "utf8 pair",
        tuple((
            map_res(length_data(be_u16), str::from_utf8),
            map_res(length_data(be_u16), str::from_utf8),
        )),
    )(input)
}

fn expiry<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Expiry, E> {
    context(
        "expiry",
        map(be_u32, |secs| match secs {
            u32::MAX => Expiry::Never,
            n => Expiry::Interval(Duration::from_secs(u64::from(n))),
        }),
    )(input)
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
    context("packet id", be_u16)(input)
}

fn reason_code<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], ReasonCode, E> {
    context("reason code", map_res(be_u8, ReasonCode::try_from))(input)
}

fn properties<'a, E: ParseError<&'a [u8]>>(
    input: &'a [u8],
) -> IResult<&'a [u8], Vec<Property<'a>>, E> {
    let (input, props) = length_data(varint)(input)?;
    let (_, props) = all_consuming(many1(property))(props)?;

    Ok((input, props))
}

fn property<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Property<'a>, E> {
    let (input, prop_id) = map_res(varint, |n| PropertyId::try_from(n as u8))(input)?;

    match prop_id {
        PropertyId::PayloadFormat => map(
            map_res(be_u8, PayloadFormat::try_from),
            Property::PayloadFormat,
        )(input),
        PropertyId::MessageExpiryInterval => map(expiry, Property::MessageExpiryInterval)(input),
        PropertyId::ContentType => map(utf8_str, Property::ContentType)(input),
        PropertyId::ResponseTopic => map(utf8_str, Property::ResponseTopic)(input),
        PropertyId::CorrelationData => map(binary_data, Property::CorrelationData)(input),
        PropertyId::SubscriptionId => map(varint, Property::SubscriptionId)(input),
        PropertyId::SessionExpiryInterval => map(expiry, Property::SessionExpiryInterval)(input),
        PropertyId::AssignedClientId => map(utf8_str, Property::AssignedClientId)(input),
        PropertyId::ServerKeepAlive => map(be_u16, Property::ServerKeepAlive)(input),
        PropertyId::AuthMethod => map(utf8_str, Property::AuthMethod)(input),
        PropertyId::AuthData => map(binary_data, Property::AuthData)(input),
        PropertyId::RequestProblem => map(be_u8, Property::RequestProblem)(input),
        PropertyId::WillDelayInterval => map(expiry, Property::WillDelayInterval)(input),
        PropertyId::RequestResponse => map(be_u8, Property::RequestResponse)(input),
        PropertyId::Response => map(utf8_str, Property::Response)(input),
        PropertyId::ServerReference => map(utf8_str, Property::ServerReference)(input),
        PropertyId::Reason => map(utf8_str, Property::Reason)(input),
        PropertyId::ReceiveMaximum => map(be_u16, Property::ReceiveMaximum)(input),
        PropertyId::TopicAliasMaximum => map(be_u16, Property::TopicAliasMaximum)(input),
        PropertyId::TopicAlias => map(be_u16, Property::TopicAlias)(input),
        PropertyId::MaximumQoS => map(map_res(be_u8, QoS::try_from), Property::MaximumQoS)(input),
        PropertyId::RetainAvailable => map(be_u8, |b| Property::RetainAvailable(b != 0))(input),
        PropertyId::UserProperty => map(utf8_str_pair, |(name, value)| {
            Property::UserProperty(name, value)
        })(input),
        PropertyId::MaximumPacketSize => map(be_u32, Property::MaximumPacketSize)(input),
        PropertyId::WildcardSubscriptionAvailable => {
            map(be_u8, |b| Property::WildcardSubscriptionAvailable(b != 0))(input)
        }
        PropertyId::SubscriptionIdAvailable => {
            map(be_u8, |b| Property::SubscriptionIdAvailable(b != 0))(input)
        }
        PropertyId::SharedSubscriptionAvailable => {
            map(be_u8, |b| Property::SharedSubscriptionAvailable(b != 0))(input)
        }
    }
}

impl Packet<'_> {
    /// Parses the bytes slice into Packet type.
    pub fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
    ) -> IResult<&'a [u8], Packet<'a>, E> {
        let (input, fixed_header) = FixedHeader::parse(input)?;
        let (input, remaining) = take(fixed_header.remaining_length)(input)?;

        match fixed_header.packet_type {
            Type::CONNECT => context(
                "Connect",
                all_consuming(map(Connect::parse, Packet::Connect)),
            )(remaining),
            Type::CONNACK => context(
                "ConnectAck",
                all_consuming(map(
                    |input| ConnectAck::parse(input, protocol_version),
                    Packet::ConnectAck,
                )),
            )(remaining),
            Type::PUBLISH => context(
                "Publish",
                all_consuming(map(
                    |input| {
                        Publish::parse(
                            input,
                            protocol_version,
                            PublishFlags::from_bits_truncate(fixed_header.packet_flags),
                        )
                    },
                    Packet::Publish,
                )),
            )(remaining),
            Type::PUBACK => context(
                "PublishAck",
                all_consuming(map(
                    |input| PublishAck::parse(input, protocol_version),
                    Packet::PublishAck,
                )),
            )(remaining),
            Type::PUBREC => context(
                "PublishReceived",
                all_consuming(map(
                    |input| PublishReceived::parse(input, protocol_version),
                    Packet::PublishReceived,
                )),
            )(remaining),
            Type::PUBREL => context(
                "PublishRelease",
                all_consuming(map(
                    |input| PublishRelease::parse(input, protocol_version),
                    Packet::PublishRelease,
                )),
            )(remaining),
            Type::PUBCOMP => context(
                "PublishComplete",
                all_consuming(map(
                    |input| PublishComplete::parse(input, protocol_version),
                    Packet::PublishComplete,
                )),
            )(remaining),
            Type::SUBSCRIBE => context(
                "Subscribe",
                all_consuming(map(
                    |input| Subscribe::parse(input, protocol_version),
                    Packet::Subscribe,
                )),
            )(remaining),
            Type::SUBACK => context(
                "SubscribeAck",
                all_consuming(map(
                    |input| SubscribeAck::parse(input, protocol_version),
                    Packet::SubscribeAck,
                )),
            )(remaining),
            Type::UNSUBSCRIBE => context(
                "Unsubscribe",
                all_consuming(map(
                    |input| Unsubscribe::parse(input, protocol_version),
                    Packet::Unsubscribe,
                )),
            )(remaining),
            Type::UNSUBACK => context(
                "UnsubscribeAck",
                all_consuming(map(
                    |input| UnsubscribeAck::parse(input, protocol_version),
                    Packet::UnsubscribeAck,
                )),
            )(remaining),
            Type::PINGREQ => context("Ping", map(all_consuming(rest), |_| Packet::Ping))(remaining),
            Type::PINGRESP => {
                context("Pong", map(all_consuming(rest), |_| Packet::Pong))(remaining)
            }
            Type::DISCONNECT => context(
                "Disconnect",
                all_consuming(map(
                    |input| Disconnect::parse(input, protocol_version),
                    Packet::Disconnect,
                )),
            )(remaining),
            Type::AUTH => context(
                "Auth",
                all_consuming(map(|input| Auth::parse(input), Packet::Auth)),
            )(remaining),
        }
        .map(|(_, packet)| (input, packet))
    }
}

impl Connect<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Connect<'a>, E> {
        let (input, (_, protocol_version, flags, keep_alive)) = tuple((
            context("protocol name", tag(PROTOCOL_NAME)),
            context(
                "protocol version",
                map_res(be_u8, ProtocolVersion::try_from),
            ),
            context("flags", map_opt(be_u8, ConnectFlags::from_bits)),
            context("keepalive", be_u16),
        ))(input)?;

        let (input, (properties, client_id, last_will, username, password)) = tuple((
            cond(protocol_version >= ProtocolVersion::V5, properties),
            client_id,
            cond(
                flags.contains(ConnectFlags::LAST_WILL),
                map(
                    tuple((
                        context("will topic", utf8_str),
                        context("will message", binary_data),
                    )),
                    |(topic_name, message)| LastWill {
                        qos: flags.qos(),
                        retain: flags.contains(ConnectFlags::WILL_RETAIN),
                        topic_name,
                        message,
                    },
                ),
            ),
            cond(
                flags.contains(ConnectFlags::USERNAME),
                context("username", utf8_str),
            ),
            cond(
                flags.contains(ConnectFlags::PASSWORD),
                context("password", binary_data),
            ),
        ))(input)?;

        Ok((
            input,
            Connect {
                protocol_version,
                clean_session: flags.contains(ConnectFlags::CLEAN_SESSION),
                keep_alive,
                properties,
                client_id,
                last_will,
                username,
                password,
            },
        ))
    }
}

impl ConnectAck<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
    ) -> IResult<&'a [u8], ConnectAck<'a>, E> {
        map(
            tuple((
                context("flags", map_opt(be_u8, ConnectAckFlags::from_bits)),
                context("return code", map_res(be_u8, ConnectReturnCode::try_from)),
                cond(protocol_version >= ProtocolVersion::V5, properties),
            )),
            |(flags, return_code, properties)| ConnectAck {
                session_present: flags.contains(ConnectAckFlags::SESSION_PRESENT),
                return_code,
                properties,
            },
        )(input)
    }
}

impl Publish<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
        flags: PublishFlags,
    ) -> IResult<&'a [u8], Publish<'a>, E> {
        let dup = flags.contains(PublishFlags::DUP);
        let qos = flags.qos();
        let retain = flags.contains(PublishFlags::RETAIN);
        let (input, (topic_name, packet_id, properties, payload)) = tuple((
            topic_name,
            cond(qos >= QoS::AtLeastOnce, packet_id),
            cond(protocol_version >= ProtocolVersion::V5, properties),
            rest,
        ))(input)?;

        Ok((
            input,
            Publish {
                dup,
                qos,
                retain,
                topic_name,
                packet_id,
                properties,
                payload,
            },
        ))
    }
}

impl PublishAck<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
    ) -> IResult<&'a [u8], PublishAck<'a>, E> {
        map(
            tuple((
                packet_id,
                opt(reason_code),
                cond(protocol_version >= ProtocolVersion::V5, properties),
            )),
            |(packet_id, reason_code, properties)| PublishAck {
                packet_id,
                reason_code,
                properties,
            },
        )(input)
    }
}

impl PublishReceived<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
    ) -> IResult<&'a [u8], PublishReceived<'a>, E> {
        map(
            tuple((
                packet_id,
                opt(reason_code),
                cond(protocol_version >= ProtocolVersion::V5, properties),
            )),
            |(packet_id, reason_code, properties)| PublishReceived {
                packet_id,
                reason_code,
                properties,
            },
        )(input)
    }
}

impl PublishRelease<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
    ) -> IResult<&'a [u8], PublishRelease<'a>, E> {
        map(
            tuple((
                packet_id,
                opt(reason_code),
                cond(protocol_version >= ProtocolVersion::V5, properties),
            )),
            |(packet_id, reason_code, properties)| PublishRelease {
                packet_id,
                reason_code,
                properties,
            },
        )(input)
    }
}

impl PublishComplete<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
    ) -> IResult<&'a [u8], PublishComplete<'a>, E> {
        map(
            tuple((
                packet_id,
                opt(reason_code),
                cond(protocol_version >= ProtocolVersion::V5, properties),
            )),
            |(packet_id, reason_code, properties)| PublishComplete {
                packet_id,
                reason_code,
                properties,
            },
        )(input)
    }
}

impl Subscribe<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
    ) -> IResult<&'a [u8], Subscribe<'a>, E> {
        map(
            tuple((
                packet_id,
                cond(protocol_version >= ProtocolVersion::V5, properties),
                many1(subscription),
            )),
            |(packet_id, properties, subscriptions)| Subscribe {
                packet_id,
                properties,
                subscriptions,
            },
        )(input)
    }
}

impl SubscribeAck<'_> {
    const QOS_MASK: u8 = 0x3;

    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
    ) -> IResult<&'a [u8], SubscribeAck<'a>, E> {
        map(
            tuple((
                packet_id,
                cond(protocol_version >= ProtocolVersion::V5, properties),
                many1(context(
                    "return code",
                    map(be_u8, |b| {
                        if (b & SubscribeReturnCode::FAILURE) == 0 {
                            SubscribeReturnCode::Success(unsafe {
                                QoS::from_unchecked(b & Self::QOS_MASK)
                            })
                        } else {
                            SubscribeReturnCode::Failure
                        }
                    }),
                )),
            )),
            |(packet_id, properties, status)| SubscribeAck {
                packet_id,
                properties,
                status,
            },
        )(input)
    }
}

impl Unsubscribe<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
    ) -> IResult<&'a [u8], Unsubscribe<'a>, E> {
        map(
            tuple((
                packet_id,
                cond(protocol_version >= ProtocolVersion::V5, properties),
                many1(topic_filter),
            )),
            |(packet_id, properties, topic_filters)| Unsubscribe {
                packet_id,
                properties,
                topic_filters,
            },
        )(input)
    }
}

impl UnsubscribeAck<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
    ) -> IResult<&'a [u8], UnsubscribeAck<'a>, E> {
        map(
            tuple((
                packet_id,
                cond(protocol_version >= ProtocolVersion::V5, properties),
            )),
            |(packet_id, properties)| UnsubscribeAck {
                packet_id,
                properties,
            },
        )(input)
    }
}

impl Disconnect<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(
        input: &'a [u8],
        protocol_version: ProtocolVersion,
    ) -> IResult<&'a [u8], Disconnect<'a>, E> {
        map(
            tuple((
                opt(reason_code),
                cond(protocol_version >= ProtocolVersion::V5, properties),
            )),
            |(reason_code, properties)| Disconnect {
                reason_code,
                properties,
            },
        )(input)
    }
}

impl Auth<'_> {
    fn parse<'a, E: ParseError<&'a [u8]>>(input: &'a [u8]) -> IResult<&'a [u8], Auth<'a>, E> {
        map(
            tuple((opt(reason_code), opt(properties))),
            |(reason_code, properties)| Auth {
                reason_code,
                properties,
            },
        )(input)
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
    fn test_varint() {
        macro_rules! assert_varint (
            ($bytes:expr, $res:expr) => {{
                assert_eq!(varint::<()>($bytes), Ok((&b""[..], $res)));
            }};

            ($bytes:expr, $res:expr, $rest:expr) => {{
                assert_eq!(varint::<()>($bytes), Ok((&$rest[..], $res)));
            }};
        );

        assert_varint!(b"\x7f\x7f", 127, b"\x7f");

        assert_eq!(
            varint(b"\xff\xff\xff"),
            Err(nom::Err::Error((&b""[..], Eof))),
            "incomplete variable length"
        );
        assert_eq!(
            varint(b"\xff\xff\xff\xff\xff\xff"),
            Err(nom::Err::Error((&b"\xff\xff\xff"[..], Verify))),
            "too long variable length"
        );

        assert_varint!(b"\x00", 0);
        assert_varint!(b"\x7f", 127);
        assert_varint!(b"\x80\x01", 128);
        assert_varint!(b"\xff\x7f", 16383);
        assert_varint!(b"\x80\x80\x01", 16384);
        assert_varint!(b"\xff\xff\x7f", 2097151);
        assert_varint!(b"\x80\x80\x80\x01", 2097152);
        assert_varint!(b"\xff\xff\xff\x7f", 268435455);
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
                    protocol_version: ProtocolVersion::V311,
                    clean_session: false,
                    keep_alive: 60,
                    properties: None,
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
                    protocol_version: ProtocolVersion::V311,
                    clean_session: false,
                    keep_alive: 60,
                    properties: None,
                    client_id: "12345",
                    last_will: Some(LastWill {
                        qos: QoS::ExactlyOnce,
                        retain: false,
                        topic_name: "topic",
                        message: b"message",
                    }),
                    username: None,
                    password: None,
                },
            ))
        );

        assert_eq!(
            Connect::parse::<()>(
                b"\x00\x04MQTT\x05\x14\x00\x3C\x0A\x11\xff\xff\xff\xff\x27\x00\x00\x10\x00\x00\x0512345\x00\x05topic\x00\x07message"
            ),
            Ok((
                &b""[..],
                Connect {
                    protocol_version: ProtocolVersion::V5,
                    clean_session: false,
                    keep_alive: 60,
                    properties: Some(vec![
                        Property::SessionExpiryInterval(Expiry::Never),
                        Property::MaximumPacketSize(4096),
                    ]),
                    client_id: "12345",
                    last_will: Some(LastWill {
                        qos: QoS::ExactlyOnce,
                        retain: false,
                        topic_name: "topic",
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
                    (&b"\x03"[..], Nom(MapRes)),
                    (&b"\x03"[..], Context("protocol version"))
                ],
            })),
            "invalid protocol version"
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
            ConnectAck::parse::<()>(b"\x01\x04", ProtocolVersion::V311),
            Ok((
                &b""[..],
                ConnectAck {
                    session_present: true,
                    return_code: ConnectReturnCode::BadUserNameOrPassword,
                    properties: None,
                }
            ))
        );

        assert_eq!(
            ConnectAck::parse(b"\x03\x04", ProtocolVersion::V311),
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
            Packet::parse::<()>(b"\xe0\x00", ProtocolVersion::V311),
            Ok((
                &b""[..],
                Packet::Disconnect(Disconnect {
                    reason_code: None,
                    properties: None
                })
            ))
        );
    }

    #[test]
    fn test_publish() {
        assert_eq!(
            Publish::parse::<()>(
                b"\x00\x05topic\x12\x34hello",
                ProtocolVersion::V311,
                QoS::AtLeastOnce.into()
            ),
            Ok((
                &b""[..],
                Publish {
                    dup: false,
                    qos: QoS::AtLeastOnce,
                    retain: false,
                    topic_name: "topic",
                    packet_id: Some(0x1234),
                    properties: None,
                    payload: b"hello",
                }
            ))
        );

        assert_eq!(
            Packet::parse::<()>(b"\x3d\x0D\x00\x05topic\x43\x21data", ProtocolVersion::V311),
            Ok((
                &b""[..],
                Packet::Publish(Publish {
                    dup: true,
                    retain: true,
                    qos: QoS::ExactlyOnce,
                    topic_name: "topic",
                    packet_id: Some(0x4321),
                    properties: None,
                    payload: b"data",
                }),
            ))
        );
        assert_eq!(
            Packet::parse::<()>(b"\x30\x0b\x00\x05topicdata", ProtocolVersion::V311),
            Ok((
                &b""[..],
                Packet::Publish(Publish {
                    dup: false,
                    retain: false,
                    qos: QoS::AtMostOnce,
                    topic_name: "topic",
                    packet_id: None,
                    properties: None,
                    payload: b"data",
                }),
            ))
        );

        assert_eq!(
            Packet::parse::<()>(b"\x40\x02\x43\x21", ProtocolVersion::V311),
            Ok((
                &b""[..],
                Packet::PublishAck(PublishAck {
                    packet_id: 0x4321,
                    reason_code: None,
                    properties: None,
                })
            ))
        );
        assert_eq!(
            Packet::parse::<()>(b"\x50\x02\x43\x21", ProtocolVersion::V311),
            Ok((
                &b""[..],
                Packet::PublishReceived(PublishReceived {
                    packet_id: 0x4321,
                    reason_code: None,
                    properties: None
                })
            ))
        );
        assert_eq!(
            Packet::parse::<()>(b"\x62\x02\x43\x21", ProtocolVersion::V311),
            Ok((
                &b""[..],
                Packet::PublishRelease(PublishRelease {
                    packet_id: 0x4321,
                    reason_code: None,
                    properties: None
                })
            ))
        );
        assert_eq!(
            Packet::parse::<()>(b"\x70\x02\x43\x21", ProtocolVersion::V311),
            Ok((
                &b""[..],
                Packet::PublishComplete(PublishComplete {
                    packet_id: 0x4321,
                    reason_code: None,
                    properties: None
                })
            ))
        );
    }

    #[test]
    fn test_subscribe() {
        assert_eq!(
            Subscribe::parse::<()>(
                b"\x12\x34\x00\x04test\x01\x00\x06filter\x02",
                ProtocolVersion::V311
            ),
            Ok((
                &b""[..],
                Subscribe {
                    packet_id: 0x1234,
                    properties: None,
                    subscriptions: vec![("test", QoS::AtLeastOnce), ("filter", QoS::ExactlyOnce)],
                }
            ))
        );
        assert_eq!(
            Packet::parse::<()>(
                b"\x82\x12\x12\x34\x00\x04test\x01\x00\x06filter\x02",
                ProtocolVersion::V311
            ),
            Ok((
                &b""[..],
                Packet::Subscribe(Subscribe {
                    packet_id: 0x1234,
                    properties: None,
                    subscriptions: vec![("test", QoS::AtLeastOnce), ("filter", QoS::ExactlyOnce)],
                })
            ))
        );

        assert_eq!(
            SubscribeAck::parse::<()>(b"\x12\x34\x01\x80\x02", ProtocolVersion::V311),
            Ok((
                &b""[..],
                SubscribeAck {
                    packet_id: 0x1234,
                    properties: None,
                    status: vec![
                        SubscribeReturnCode::Success(QoS::AtLeastOnce),
                        SubscribeReturnCode::Failure,
                        SubscribeReturnCode::Success(QoS::ExactlyOnce),
                    ],
                }
            ))
        );

        assert_eq!(
            Packet::parse::<()>(b"\x90\x05\x12\x34\x01\x80\x02", ProtocolVersion::V311),
            Ok((
                &b""[..],
                Packet::SubscribeAck(SubscribeAck {
                    packet_id: 0x1234,
                    properties: None,
                    status: vec![
                        SubscribeReturnCode::Success(QoS::AtLeastOnce),
                        SubscribeReturnCode::Failure,
                        SubscribeReturnCode::Success(QoS::ExactlyOnce),
                    ],
                })
            ))
        );

        assert_eq!(
            Unsubscribe::parse::<()>(b"\x12\x34\x00\x04test\x00\x06filter", ProtocolVersion::V311),
            Ok((
                &b""[..],
                Unsubscribe {
                    packet_id: 0x1234,
                    properties: None,
                    topic_filters: vec!["test", "filter"],
                }
            ))
        );
        assert_eq!(
            Packet::parse::<()>(
                b"\xa2\x10\x12\x34\x00\x04test\x00\x06filter",
                ProtocolVersion::V311
            ),
            Ok((
                &b""[..],
                Packet::Unsubscribe(Unsubscribe {
                    packet_id: 0x1234,
                    properties: None,
                    topic_filters: vec!["test", "filter"],
                })
            ))
        );

        assert_eq!(
            Packet::parse::<()>(b"\xb0\x02\x43\x21", ProtocolVersion::V311),
            Ok((
                &b""[..],
                Packet::UnsubscribeAck(UnsubscribeAck {
                    packet_id: 0x4321,
                    properties: None,
                })
            ))
        );

        assert_eq!(
            Packet::parse(b"\x82\x02\x42\x42", ProtocolVersion::V311),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&[][..], Nom(Eof)),
                    (&[][..], Context("utf8 string")),
                    (&[][..], Context("topic filter")),
                    (&[][..], Context("subscription")),
                    (&[][..], Nom(Many1)),
                    (b"\x42\x42", Context("Subscribe")),
                ]
            })),
            "subscribe without subscription topics"
        );

        assert_eq!(
            Packet::parse(b"\x82\x04\x42\x42\x00\x00", ProtocolVersion::V311),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&[][..], Nom(Eof)),
                    (&[][..], Context("QoS")),
                    (&[0, 0][..], Context("subscription")),
                    (&[0, 0][..], Nom(Many1)),
                    (b"\x42\x42\x00\x00", Context("Subscribe")),
                ]
            })),
            "malformed/malicious subscribe packets: no QoS for topic filter"
        );

        assert_eq!(
            Packet::parse(b"\x82\x03\x42\x42\x00", ProtocolVersion::V311),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&[0][..], Nom(Eof)),
                    (&[0][..], Context("utf8 string")),
                    (&[0][..], Context("topic filter")),
                    (&[0][..], Context("subscription")),
                    (&[0][..], Nom(Many1)),
                    (b"\x42\x42\x00", Context("Subscribe")),
                ]
            })),
            "truncated string length prefix"
        );

        assert_eq!(
            Packet::parse(b"\xa2\x02\x42\x42", ProtocolVersion::V311),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&[][..], Nom(Eof)),
                    (&[][..], Context("utf8 string")),
                    (&[][..], Context("topic filter")),
                    (&[][..], Nom(Many1)),
                    (b"\x42\x42", Context("Unsubscribe")),
                ]
            })),
            "unsubscribe without subscription topics"
        );

        assert_eq!(
            Packet::parse(b"\xa2\x03\x42\x42\x00", ProtocolVersion::V311),
            Err(nom::Err::Error(VerboseError {
                errors: vec![
                    (&[0][..], Nom(Eof)),
                    (&[0][..], Context("utf8 string")),
                    (&[0][..], Context("topic filter")),
                    (&[0][..], Nom(Many1)),
                    (b"\x42\x42\x00", Context("Unsubscribe")),
                ]
            })),
            "malformed/malicious unsubscribe packets: truncated string length prefix"
        );
    }

    #[test]
    fn test_ping_pong() {
        assert_eq!(
            Packet::parse::<()>(b"\xc0\x00", ProtocolVersion::V311),
            Ok((&b""[..], Packet::Ping))
        );
        assert_eq!(
            Packet::parse::<()>(b"\xd0\x00", ProtocolVersion::V311),
            Ok((&b""[..], Packet::Pong))
        );
    }
}