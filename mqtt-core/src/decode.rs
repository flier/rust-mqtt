use std::borrow::Cow;
use std::str;

use nom::{be_u16, be_u8, Context, Err, ErrorKind, IResult, Needed};

use crate::packet::*;
use crate::proto::*;

const QOS_MASK: u8 = 0x03;

pub const INVALID_PROTOCOL: u32 = 0x0001;
pub const UNSUPPORT_LEVEL: u32 = 0x0002;
pub const RESERVED_FLAG: u32 = 0x0003;
pub const INVALID_CLIENT_ID: u32 = 0x0004;
pub const INVALID_LENGTH: u32 = 0x0005;
pub const UNSUPPORT_PACKET_TYPE: u32 = 0x0100;

macro_rules! error_if (
    ($i:expr, $cond:expr, $code:expr) => ({
        if $cond {
            Err(nom::Err::Error(Context::Code($i, ErrorKind::Custom($code))))
        } else {
            Ok(($i, ()))
        }
    });
    ($i:expr, $cond:expr, $err:expr) => (
        error!($i, $cond, $err);
    );
);

pub fn decode_variable_length_usize(i: &[u8]) -> IResult<&[u8], usize> {
    let n = if i.len() > 4 { 4 } else { i.len() };
    let pos = i[..n].iter().position(|b| (b & 0x80) == 0);

    match pos {
        Some(idx) => Ok((
            &i[idx + 1..],
            i[..idx + 1].iter().enumerate().fold(0usize, |acc, (i, b)| {
                acc + (((b & 0x7F) as usize) << (i * 7))
            }),
        )),
        _ => {
            if n < 4 {
                Err(Err::Incomplete(Needed::Unknown))
            } else {
                Err(Err::Error(Context::Code(
                    i,
                    ErrorKind::Custom(INVALID_LENGTH),
                )))
            }
        }
    }
}

named!(pub decode_length_bytes, length_bytes!(be_u16));

fn decode_utf8_str(i: &[u8]) -> IResult<&[u8], Cow<str>> {
    be_u16(i).and_then(|(rest, len)| {
        let len = len as usize;

        if rest.len() < len {
            Err(Err::Incomplete(Needed::Size(len - rest.len())))
        } else {
            Ok((
                &rest[len..],
                unsafe { str::from_utf8_unchecked(&rest[..len]) }.into(),
            ))
        }
    })
}

named!(pub decode_fixed_header<FixedHeader>, do_parse!(
    b0: bits!( pair!( take_bits!( u8, 4 ), take_bits!( u8, 4 ) ) ) >>
    remaining_length: decode_variable_length_usize >>
    (
        FixedHeader {
            packet_type: b0.0.into(),
            packet_flags: b0.1,
            remaining_length: remaining_length,
        }
    )
));

macro_rules! is_flag_set {
    ($flags:expr, $flag:expr) => {
        ($flags & $flag.bits()) == $flag.bits()
    };
}

named!(pub decode_connect_header<Packet>, do_parse!(
    len: be_u16 >> error_if!(len != 4, INVALID_PROTOCOL) >>
    proto: take!(4) >> error_if!(proto != b"MQTT", INVALID_PROTOCOL) >>
    level: be_u8 >> error_if!(level != DEFAULT_MQTT_LEVEL, UNSUPPORT_LEVEL) >>

    // The Server MUST validate that the reserved flag in the CONNECT Control Packet is set to zero
    // and disconnect the Client if it is not zero [MQTT-3.1.2-3].
    flags: be_u8 >> error_if!((flags & 0x01) != 0, RESERVED_FLAG) >>
    keep_alive: be_u16 >>

    client_id: decode_utf8_str >> error_if!(client_id.is_empty() && !is_flag_set!(flags, ConnectFlags::CLEAN_SESSION), INVALID_CLIENT_ID) >>

    topic: cond!(is_flag_set!(flags, ConnectFlags::WILL), decode_utf8_str) >>
    message: cond!(is_flag_set!(flags, ConnectFlags::WILL), decode_length_bytes) >>
    username: cond!(is_flag_set!(flags, ConnectFlags::USERNAME), decode_utf8_str) >>
    password: cond!(is_flag_set!(flags, ConnectFlags::PASSWORD), decode_length_bytes) >>
    (
        Packet::Connect {
            protocol: Protocol::MQTT(level),
            clean_session: is_flag_set!(flags, ConnectFlags::CLEAN_SESSION),
            keep_alive: keep_alive,
            client_id: client_id,
            last_will: if is_flag_set!(flags, ConnectFlags::WILL) { Some(LastWill{
                qos: QoS::from((flags & ConnectFlags::WILL_QOS.bits()) >> WILL_QOS_SHIFT),
                retain: is_flag_set!(flags, ConnectFlags::WILL_RETAIN),
                topic: topic.unwrap(),
                message: message.unwrap().into(),
            }) } else { None },
            username: username.into(),
            password: password.map(Cow::from),
        }
    )
));

named!(pub decode_connect_ack_header<(ConnectAckFlags, ConnectReturnCode)>, do_parse!(
    // Byte 1 is the "Connect Acknowledge Flags". Bits 7-1 are reserved and MUST be set to 0.
    flags: add_return_error!(ErrorKind::Custom(RESERVED_FLAG), verify!(be_u8, |flags| (flags & 0b1111_1110) == 0)) >>

    return_code: be_u8 >>
    (
        ConnectAckFlags::from_bits_truncate(flags), // Bit 0 (SP1) is the Session Present Flag.
        ConnectReturnCode::from(return_code)
    )
));

pub fn decode_publish_header(i: &[u8]) -> IResult<&[u8], (Cow<str>, u16)> {
    decode_utf8_str(i)
        .and_then(|(rest, topic)| be_u16(rest).map(|(rest, packet_id)| (rest, (topic, packet_id))))
}

named!(pub decode_subscribe_header<Packet>, do_parse!(
    packet_id: be_u16 >>
    topic_filters: many1!(complete!(pair!(decode_utf8_str, be_u8))) >>
    (
        Packet::Subscribe {
            packet_id: packet_id,
            topic_filters: topic_filters.into_iter()
                                        .map(|(filter, flags)| (filter, QoS::from(flags & QOS_MASK)))
                                        .collect(),
        }
    )
));

named!(pub decode_subscribe_ack_header<Packet>, do_parse!(
    packet_id: be_u16 >>
    return_codes: many1!(complete!(be_u8)) >>
    (
        Packet::SubscribeAck {
            packet_id: packet_id,
            status: return_codes.iter()
                                .map(|&return_code| if return_code == 0x80 {
                                    SubscribeReturnCode::Failure
                                } else {
                                    SubscribeReturnCode::Success(QoS::from(return_code & QOS_MASK))
                                })
                                .collect(),
        }
    )
));

named!(pub decode_unsubscribe_header<Packet>, do_parse!(
    packet_id: be_u16 >>
    topic_filters: many1!(complete!(decode_utf8_str)) >>
    (
        Packet::Unsubscribe {
            packet_id: packet_id,
            topic_filters: topic_filters,
        }
    )
));

fn decode_variable_header(i: &[u8], fixed_header: FixedHeader) -> IResult<&[u8], Packet> {
    match fixed_header.packet_type.into() {
        PacketType::CONNECT => decode_connect_header(i),
        PacketType::CONNACK => decode_connect_ack_header(i).map(|(rest, (flags, return_code))| {
            (
                rest,
                Packet::ConnectAck {
                    session_present: is_flag_set!(flags.bits(), ConnectAckFlags::SESSION_PRESENT),
                    return_code: return_code,
                },
            )
        }),
        PacketType::PUBLISH => {
            let dup = (fixed_header.packet_flags & 0b1000) == 0b1000;
            let qos = QoS::from((fixed_header.packet_flags & 0b0110) >> 1);
            let retain = (fixed_header.packet_flags & 0b0001) == 0b0001;

            let result = match qos {
                QoS::AtLeastOnce | QoS::ExactlyOnce => decode_publish_header(i)
                    .map(|(rest, (topic, packet_id))| (rest, (topic, Some(packet_id)))),
                QoS::AtMostOnce => decode_utf8_str(i).map(|(rest, topic)| (rest, (topic, None))),
                // A PUBLISH Packet MUST NOT have both QoS bits set to 1.
                // If a Server or Client receives a PUBLISH Packet
                // which has both QoS bits set to 1 it MUST close the Network Connection [MQTT-3.3.1-4].
                QoS::Reserved => Err(Err::Error(Context::Code(
                    i,
                    ErrorKind::Custom(RESERVED_FLAG),
                ))),
            };

            result.map(|(i, (topic, packet_id))| {
                (
                    Default::default(),
                    Packet::Publish {
                        dup: dup,
                        retain: retain,
                        qos: qos,
                        topic: topic,
                        packet_id: packet_id,
                        payload: i.into(),
                    },
                )
            })
        }
        PacketType::PUBACK => {
            be_u16(i).map(|(rest, packet_id)| (rest, Packet::PublishAck { packet_id }))
        }
        PacketType::PUBREC => {
            be_u16(i).map(|(rest, packet_id)| (rest, Packet::PublishReceived { packet_id }))
        }
        PacketType::PUBREL => {
            // Bits 3,2,1 and 0 of the fixed header in the PUBREL Control Packet are reserved and MUST be set to 0,0,1 and 0 respectively.
            // The Server MUST treat any other value as malformed and close the Network Connection [MQTT-3.6.1-1].
            if fixed_header.packet_flags == 0b0010 {
                be_u16(i).map(|(rest, packet_id)| (rest, Packet::PublishRelease { packet_id }))
            } else {
                Err(Err::Error(Context::Code(
                    i,
                    ErrorKind::Custom(RESERVED_FLAG),
                )))
            }
        }
        PacketType::PUBCOMP => {
            be_u16(i).map(|(rest, packet_id)| (rest, Packet::PublishComplete { packet_id }))
        }

        PacketType::SUBSCRIBE => {
            // Bits 3,2,1 and 0 of the fixed header in the PUBREL Control Packet are reserved and MUST be set to 0,0,1 and 0 respectively.
            // The Server MUST treat any other value as malformed and close the Network Connection [MQTT-3.6.1-1].
            if fixed_header.packet_flags == 0b0010 {
                decode_subscribe_header(i)
            } else {
                Err(Err::Error(Context::Code(
                    i,
                    ErrorKind::Custom(RESERVED_FLAG),
                )))
            }
        }
        PacketType::SUBACK => decode_subscribe_ack_header(i),
        PacketType::UNSUBSCRIBE => {
            // Bits 3,2,1 and 0 of the fixed header in the PUBREL Control Packet are reserved and MUST be set to 0,0,1 and 0 respectively.
            // The Server MUST treat any other value as malformed and close the Network Connection [MQTT-3.6.1-1].
            if fixed_header.packet_flags == 0b0010 {
                decode_unsubscribe_header(i)
            } else {
                Err(Err::Error(Context::Code(
                    i,
                    ErrorKind::Custom(RESERVED_FLAG),
                )))
            }
        }
        PacketType::UNSUBACK => {
            be_u16(i).map(|(rest, packet_id)| (rest, Packet::UnsubscribeAck { packet_id }))
        }

        PacketType::PINGREQ => Ok((i, Packet::PingRequest)),
        PacketType::PINGRESP => Ok((i, Packet::PingResponse)),
        PacketType::DISCONNECT => {
            if fixed_header.packet_flags == 0 {
                Ok((i, Packet::Disconnect))
            } else {
                Err(Err::Error(Context::Code(
                    i,
                    ErrorKind::Custom(RESERVED_FLAG),
                )))
            }
        }
        _ => {
            let err_code = UNSUPPORT_PACKET_TYPE + (fixed_header.packet_type as u32);
            Err(Err::Error(Context::Code(i, ErrorKind::Custom(err_code))))
        }
    }
}

named!(pub decode_packet<Packet>, do_parse!(
    fixed_header: decode_fixed_header >>
    packet: flat_map!(
        take!(fixed_header.remaining_length),
        complete!(apply!(decode_variable_header, fixed_header))
    ) >>
    ( packet )
));

/// Extends `AsRef<[u8]>` with methods for reading packet.
///
/// ```
/// use mqtt_core::{ReadPacketExt, Packet};
///
/// assert_eq!(b"\xd0\x00".read_packet().unwrap(), Packet::PingResponse);
/// ```
pub trait ReadPacketExt: AsRef<[u8]> {
    #[inline]
    /// Read packet from the underlying reader.
    fn read_packet(&self) -> Result<Packet, Err<&[u8]>> {
        decode_packet(self.as_ref()).map(|(_, packet)| packet)
    }
}

impl<T: AsRef<[u8]>> ReadPacketExt for T {}

/// Read packet from the underlying `&[u8]`.
///
/// ```
/// use mqtt_core::{read_packet, Packet};
///
/// assert_eq!(read_packet(b"\xc0\x00\xd0\x00").unwrap(), (&b"\xd0\x00"[..], Packet::PingRequest));
/// ```
pub fn read_packet(i: &[u8]) -> Result<(&[u8], Packet), Err<&[u8]>> {
    decode_packet(i)
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use nom::{ErrorKind, Needed};

    use super::*;

    #[test]
    fn test_decode_variable_length() {
        macro_rules! assert_variable_length (
            ($bytes:expr, $res:expr) => {{
                assert_eq!(decode_variable_length_usize($bytes), Ok((&b""[..], $res)));
            }};

            ($bytes:expr, $res:expr, $rest:expr) => {{
                assert_eq!(decode_variable_length_usize($bytes), Ok((&$rest[..], $res)));
            }};
        );

        assert_variable_length!(b"\x7f\x7f", 127, b"\x7f");

        assert_eq!(
            decode_variable_length_usize(b"\xff\xff\xff"),
            Err(Err::Incomplete(Needed::Unknown))
        );
        assert_matches!(
            decode_variable_length_usize(b"\xff\xff\xff\xff\xff\xff"),
            Err(Err::Error(Context::Code(_, ErrorKind::Custom(INVALID_LENGTH))))
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
    fn test_decode_fixed_header() {
        assert_eq!(
            decode_fixed_header(b"\x20\x7f"),
            Ok((
                &b""[..],
                FixedHeader {
                    packet_type: PacketType::CONNACK,
                    packet_flags: 0,
                    remaining_length: 127,
                },
            ))
        );

        assert_eq!(
            decode_fixed_header(b"\x3C\x82\x7f"),
            Ok((
                &b""[..],
                FixedHeader {
                    packet_type: PacketType::PUBLISH,
                    packet_flags: 0x0C,
                    remaining_length: 16258,
                },
            ))
        );

        assert_eq!(
            decode_fixed_header(b"\x20"),
            Err(Err::Incomplete(Needed::Unknown))
        );
    }

    #[test]
    fn test_decode_connect_packets() {
        assert_eq!(
            decode_connect_header(
                b"\x00\x04MQTT\x04\xC0\x00\x3C\x00\x0512345\x00\x04user\x00\x04pass",
            ),
            Ok((
                &b""[..],
                Packet::Connect {
                    protocol: Protocol::MQTT(4),
                    clean_session: false,
                    keep_alive: 60,
                    client_id: Cow::from("12345"),
                    last_will: None,
                    username: Some(Cow::from("user")),
                    password: Some(Cow::from(&b"pass"[..])),
                },
            ))
        );

        assert_eq!(
            decode_connect_header(
                b"\x00\x04MQTT\x04\x14\x00\x3C\x00\x0512345\x00\x05topic\x00\x07message",
            ),
            Ok((
                &b""[..],
                Packet::Connect {
                    protocol: Protocol::MQTT(4),
                    clean_session: false,
                    keep_alive: 60,
                    client_id: Cow::from("12345"),
                    last_will: Some(LastWill {
                        qos: QoS::ExactlyOnce,
                        retain: false,
                        topic: Cow::from("topic"),
                        message: Cow::from(&b"message"[..]),
                    }),
                    username: None,
                    password: None,
                },
            ))
        );

        assert_matches!(
            decode_connect_header(b"\x00\x02MQ"),
            Err(Err::Error(Context::Code(_, ErrorKind::Custom(INVALID_PROTOCOL))))
        );
        assert_matches!(
            decode_connect_header(b"\x00\x04MQAA"),
            Err(Err::Error(Context::Code(_, ErrorKind::Custom(INVALID_PROTOCOL))))
        );
        assert_matches!(
            decode_connect_header(b"\x00\x04MQTT\x03"),
            Err(Err::Error(Context::Code(_, ErrorKind::Custom(UNSUPPORT_LEVEL))))
        );
        assert_matches!(
            decode_connect_header(b"\x00\x04MQTT\x04\xff"),
            Err(Err::Error(Context::Code(_, ErrorKind::Custom(RESERVED_FLAG))))
        );

        assert_eq!(
            decode_connect_ack_header(b"\x01\x04"),
            Ok((
                &b""[..],
                (
                    ConnectAckFlags::SESSION_PRESENT,
                    ConnectReturnCode::BadUserNameOrPassword,
                )
            ))
        );

        assert_eq!(
            decode_connect_ack_header(b"\x03\x04"),
            Err(Err::Error(Context::List(vec![
                (&b"\x03\x04"[..], ErrorKind::Verify),
                (&b"\x03\x04"[..], ErrorKind::Custom(RESERVED_FLAG))
            ])))
        );

        assert_eq!(
            decode_packet(b"\x20\x02\x01\x04"),
            Ok((
                &b""[..],
                Packet::ConnectAck {
                    session_present: true,
                    return_code: ConnectReturnCode::BadUserNameOrPassword,
                },
            ))
        );

        assert_eq!(
            decode_packet(b"\xe0\x00"),
            Ok((&b""[..], Packet::Disconnect))
        );
    }

    #[test]
    fn test_decode_publish_packets() {
        assert_eq!(
            decode_publish_header(b"\x00\x05topic\x12\x34"),
            Ok((&b""[..], (Cow::from("topic"), 0x1234)))
        );

        assert_eq!(
            decode_packet(b"\x3d\x0D\x00\x05topic\x43\x21data"),
            Ok((
                &b""[..],
                Packet::Publish {
                    dup: true,
                    retain: true,
                    qos: QoS::ExactlyOnce,
                    topic: Cow::from("topic"),
                    packet_id: Some(0x4321),
                    payload: (&b"data"[..]).into(),
                },
            ))
        );
        assert_eq!(
            decode_packet(b"\x30\x0b\x00\x05topicdata"),
            Ok((
                &b""[..],
                Packet::Publish {
                    dup: false,
                    retain: false,
                    qos: QoS::AtMostOnce,
                    topic: Cow::from("topic"),
                    packet_id: None,
                    payload: (&b"data"[..]).into(),
                },
            ))
        );

        assert_eq!(
            decode_packet(b"\x40\x02\x43\x21"),
            Ok((&b""[..], Packet::PublishAck { packet_id: 0x4321 }))
        );
        assert_eq!(
            decode_packet(b"\x50\x02\x43\x21"),
            Ok((&b""[..], Packet::PublishReceived { packet_id: 0x4321 }))
        );
        assert_eq!(
            decode_packet(b"\x62\x02\x43\x21"),
            Ok((&b""[..], Packet::PublishRelease { packet_id: 0x4321 }))
        );
        assert_eq!(
            decode_packet(b"\x70\x02\x43\x21"),
            Ok((&b""[..], Packet::PublishComplete { packet_id: 0x4321 }))
        );
    }

    #[test]
    fn test_decode_subscribe_packets() {
        let p = Packet::Subscribe {
            packet_id: 0x1234,
            topic_filters: vec![
                (Cow::from("test"), QoS::AtLeastOnce),
                (Cow::from("filter"), QoS::ExactlyOnce),
            ],
        };

        assert_eq!(
            decode_subscribe_header(b"\x12\x34\x00\x04test\x01\x00\x06filter\x02"),
            Ok((&b""[..], p.clone()))
        );
        assert_eq!(
            decode_packet(b"\x82\x12\x12\x34\x00\x04test\x01\x00\x06filter\x02"),
            Ok((&b""[..], p))
        );

        let p = Packet::SubscribeAck {
            packet_id: 0x1234,
            status: vec![
                SubscribeReturnCode::Success(QoS::AtLeastOnce),
                SubscribeReturnCode::Failure,
                SubscribeReturnCode::Success(QoS::ExactlyOnce),
            ],
        };

        assert_eq!(
            decode_subscribe_ack_header(b"\x12\x34\x01\x80\x02"),
            Ok((&b""[..], p.clone()))
        );

        assert_eq!(
            decode_packet(b"\x90\x05\x12\x34\x01\x80\x02"),
            Ok((&b""[..], p))
        );

        let p = Packet::Unsubscribe {
            packet_id: 0x1234,
            topic_filters: vec![Cow::from("test"), Cow::from("filter")],
        };

        assert_eq!(
            decode_unsubscribe_header(b"\x12\x34\x00\x04test\x00\x06filter"),
            Ok((&b""[..], p.clone()))
        );
        assert_eq!(
            decode_packet(b"\xa2\x10\x12\x34\x00\x04test\x00\x06filter"),
            Ok((&b""[..], p))
        );

        assert_eq!(
            decode_packet(b"\xb0\x02\x43\x21"),
            Ok((&b""[..], Packet::UnsubscribeAck { packet_id: 0x4321 }))
        );
    }

    macro_rules! assert_complete (
        ($pkt:expr, $err:path) => {{
            assert_matches!($pkt, Err(Err::Error(Context::Code(_, $err))));
        }};
    );

    #[test]
    fn test_invalid_subscribe_packets() {
        // subscribe without subscription topics
        assert_complete!(decode_packet(b"\x82\x02\x42\x42"), ErrorKind::Many1);

        // malformed/malicious subscribe packets:
        // no QoS for topic filter
        assert_complete!(decode_packet(b"\x82\x04\x42\x42\x00\x00"), ErrorKind::Many1);

        // truncated string length prefix
        assert_complete!(decode_packet(b"\x82\x03\x42\x42\x00"), ErrorKind::Many1);

        // unsubscribe without subscription topics
        assert_complete!(decode_packet(b"\xa2\x02\x42\x42"), ErrorKind::Many1);

        // malformed/malicious unsubscribe packets: truncated string length prefix
        assert_complete!(decode_packet(b"\xa2\x03\x42\x42\x00"), ErrorKind::Many1);
    }

    #[test]
    fn test_decode_ping_packets() {
        assert_eq!(
            decode_packet(b"\xc0\x00"),
            Ok((&b""[..], Packet::PingRequest))
        );
        assert_eq!(
            decode_packet(b"\xd0\x00"),
            Ok((&b""[..], Packet::PingResponse))
        );
    }
}
