use core::mem;

use bytes::BufMut;

use crate::packet::*;

pub const LENGTH_FIELD_SIZE: usize = mem::size_of::<u16>();

impl Packet<'_> {
    fn fixed_header(&self) -> FixedHeader {
        FixedHeader {
            packet_type: self.packet_type(),
            packet_flags: self.packet_flags(),
            remaining_length: self.remaining_length(),
        }
    }

    /// The MQTT control packet type.
    pub fn packet_type(&self) -> Type {
        match *self {
            Packet::Connect(_) => Type::CONNECT,
            Packet::ConnectAck(_) => Type::CONNACK,
            Packet::Publish(_) => Type::PUBLISH,
            Packet::PublishAck(_) => Type::PUBACK,
            Packet::PublishReceived(_) => Type::PUBREC,
            Packet::PublishRelease(_) => Type::PUBREL,
            Packet::PublishComplete(_) => Type::PUBCOMP,
            Packet::Subscribe(_) => Type::SUBSCRIBE,
            Packet::SubscribeAck(_) => Type::SUBACK,
            Packet::Unsubscribe(_) => Type::UNSUBSCRIBE,
            Packet::UnsubscribeAck(_) => Type::UNSUBACK,
            Packet::Ping => Type::PINGREQ,
            Packet::Pong => Type::PINGRESP,
            Packet::Disconnect => Type::DISCONNECT,
        }
    }

    fn packet_flags(&self) -> u8 {
        match self {
            Packet::Publish(ref publish) => publish.flags().bits(),
            Packet::PublishRelease(_) | Packet::Subscribe(_) | Packet::Unsubscribe(_) => 0x02,
            _ => 0,
        }
    }

    fn remaining_length(&self) -> usize {
        match self {
            Packet::Connect(ref connect) => connect.size(),
            Packet::ConnectAck(ref connect_ack) => connect_ack.size(),
            Packet::Publish(ref publish) => publish.size(),
            Packet::PublishAck(ref publish_ack) => publish_ack.size(),
            Packet::PublishReceived(ref publish_received) => publish_received.size(),
            Packet::PublishRelease(ref publish_release) => publish_release.size(),
            Packet::PublishComplete(ref publish_complete) => publish_complete.size(),
            Packet::Subscribe(ref subscribe) => subscribe.size(),
            Packet::SubscribeAck(ref subscribe_ack) => subscribe_ack.size(),
            Packet::Unsubscribe(ref unsubscribe) => unsubscribe.size(),
            Packet::UnsubscribeAck(ref unsubscribe_ack) => unsubscribe_ack.size(),
            Packet::Ping | Packet::Pong | Packet::Disconnect => 0,
        }
    }
}

trait BufMutExt: BufMut {
    fn put_str(&mut self, s: &str) {
        self.put_bytes(s.as_bytes())
    }

    fn put_bytes(&mut self, s: &[u8]) {
        self.put_u16(s.len() as u16);
        self.put_slice(s)
    }

    fn put_varint(&mut self, mut n: usize) {
        loop {
            let b = (n % 0x80) as u8;
            n >>= 7;
            if n > 0 {
                self.put_u8(0x80 | b);
            } else {
                self.put_u8(b);
                break;
            }
        }
    }
}

impl<T: BufMut> BufMutExt for T {}

/// A trait for objects which can be written to byte-oriented sinks.
pub trait WriteTo {
    /// Gets the size of this object.
    fn size(&self) -> usize;

    /// Writes this object to the given byte-oriented sink.
    fn write_to<T: BufMut>(&self, buf: &mut T);
}

impl WriteTo for Packet<'_> {
    fn size(&self) -> usize {
        let fixed_header = self.fixed_header();
        fixed_header.size() + fixed_header.remaining_length
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        self.fixed_header().write_to(buf);

        match self {
            Packet::Connect(ref connect) => connect.write_to(buf),
            Packet::ConnectAck(ref connect_ack) => connect_ack.write_to(buf),
            Packet::Publish(ref publish) => publish.write_to(buf),
            Packet::PublishAck(ref publish_ack) => publish_ack.write_to(buf),
            Packet::PublishReceived(ref publish_received) => publish_received.write_to(buf),
            Packet::PublishRelease(ref publish_release) => publish_release.write_to(buf),
            Packet::PublishComplete(ref publish_complete) => publish_complete.write_to(buf),
            Packet::Subscribe(ref subscribe) => subscribe.write_to(buf),
            Packet::SubscribeAck(ref subscribe_ack) => subscribe_ack.write_to(buf),
            Packet::Unsubscribe(ref unsubscribe) => unsubscribe.write_to(buf),
            Packet::UnsubscribeAck(ref unsubscribe_ack) => unsubscribe_ack.write_to(buf),
            Packet::Ping | Packet::Pong | Packet::Disconnect => {}
        }
    }
}

impl WriteTo for FixedHeader {
    fn size(&self) -> usize {
        mem::size_of::<u8>() // packet_type + packet_flags
            + match self.remaining_length {
                n if n <= 127 => 1,         // (0x7F)
                n if n <= 16_383 => 2,      // (0xFF, 0x7F)
                n if n <= 2_097_151 => 3,   // (0xFF, 0xFF, 0x7F)
                n if n <= 268_435_455 => 4, // (0xFF, 0xFF, 0xFF, 0x7F)
                _ => panic!("remaining length {} too large", self.remaining_length),
            }
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u8(((self.packet_type as u8) << 4) + self.packet_flags);
        buf.put_varint(self.remaining_length);
    }
}

impl WriteTo for Connect<'_> {
    fn size(&self) -> usize {
        PROTOCOL_NAME.len()
            + mem::size_of::<u8>()                      // protocol_level
            + mem::size_of::<ConnectFlags>()            // flags
            + mem::size_of::<u16>()                     // keep_alive
            + LENGTH_FIELD_SIZE + self.client_id.len()  // client_id
            + self.last_will.as_ref().map_or(0, |will| {
                LENGTH_FIELD_SIZE + will.topic.len() + LENGTH_FIELD_SIZE + will.message.len()
            })
            + self.username.map_or(0, |s| LENGTH_FIELD_SIZE + s.len())
            + self.password.map_or(0, |s| LENGTH_FIELD_SIZE + s.len())
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        let mut flags = ConnectFlags::empty();
        if let Some(ref will) = self.last_will {
            flags.remove(ConnectFlags::WILL_QOS);
            flags |= ConnectFlags::LAST_WILL | will.qos.into();
            if will.retain {
                flags.insert(ConnectFlags::WILL_RETAIN);
            } else {
                flags.remove(ConnectFlags::WILL_RETAIN);
            }
        }
        if self.username.is_some() {
            flags |= ConnectFlags::USERNAME;
        }
        if self.password.is_some() {
            flags |= ConnectFlags::PASSWORD;
        }
        if self.clean_session {
            flags |= ConnectFlags::CLEAN_SESSION;
        }

        buf.put_slice(PROTOCOL_NAME);
        buf.put_u8(PROTOCOL_LEVEL);
        buf.put_u8(flags.bits());
        buf.put_u16(self.keep_alive);
        buf.put_str(self.client_id);
        if let Some(ref will) = self.last_will {
            buf.put_str(will.topic);
            buf.put_bytes(will.message);
        }
        if let Some(ref username) = self.username {
            buf.put_str(username);
        }
        if let Some(ref password) = self.password {
            buf.put_bytes(password);
        }
    }
}

impl WriteTo for ConnectAck {
    fn size(&self) -> usize {
        mem::size_of::<ConnectAckFlags>() + mem::size_of::<ConnectReturnCode>()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u8(if self.session_present {
            ConnectAckFlags::SESSION_PRESENT.bits()
        } else {
            0
        });
        buf.put_u8(self.return_code as u8);
    }
}

impl Publish<'_> {
    fn flags(&self) -> PublishFlags {
        let mut flags = PublishFlags::from(self.qos);
        if self.dup {
            flags |= PublishFlags::DUP;
        }
        if self.retain {
            flags |= PublishFlags::RETAIN;
        }
        flags
    }
}

impl WriteTo for Publish<'_> {
    fn size(&self) -> usize {
        LENGTH_FIELD_SIZE
            + self.topic.len()
            + self.packet_id.map_or(0, |_| mem::size_of::<PacketId>())
            + self.payload.len()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_str(self.topic);
        if let Some(packet_id) = self.packet_id {
            buf.put_u16(packet_id);
        }
        buf.put_slice(self.payload)
    }
}

impl WriteTo for PublishAck {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
    }
}

impl WriteTo for PublishReceived {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
    }
}

impl WriteTo for PublishRelease {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
    }
}

impl WriteTo for PublishComplete {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
    }
}

impl WriteTo for Subscribe<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self
                .subscriptions
                .iter()
                .map(|(topic_filter, _)| {
                    LENGTH_FIELD_SIZE + topic_filter.len() + mem::size_of::<QoS>()
                })
                .sum::<usize>()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);

        for &(topic_filter, qos) in &self.subscriptions {
            buf.put_str(topic_filter);
            buf.put_u8(qos as u8)
        }
    }
}

impl WriteTo for SubscribeAck {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + mem::size_of::<SubscribeReturnCode>() * self.status.iter().count()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);

        for &return_code in &self.status {
            buf.put_u8(return_code.into())
        }
    }
}

impl WriteTo for Unsubscribe<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self
                .topic_filters
                .iter()
                .map(|topic_filter| LENGTH_FIELD_SIZE + topic_filter.len())
                .sum::<usize>()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);

        for &topic_filter in &self.topic_filters {
            buf.put_str(topic_filter);
        }
    }
}

impl WriteTo for UnsubscribeAck {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoded_data() {
        let mut v = Vec::new();

        v.put_str("hello");
        v.put_bytes(b"world");
        v.put_varint(123);
        v.put_varint(129);
        v.put_varint(16383);
        v.put_varint(2097151);
        v.put_varint(268435455);

        assert_eq!(
            v,
            b"\x00\x05hello\x00\x05world\x7b\x81\x01\xff\x7f\xff\xff\x7f\xff\xff\xff\x7f"
        );
    }

    macro_rules! assert_packet {
        ($packet:expr, $right:expr) => {
            assert_eq!($packet.size(), $right.len(), "assert packet size");

            let mut v = Vec::new();
            $packet.write_to(&mut v);
            assert_eq!(v, &$right[..], "assert packet content: {:#?}", $packet);
        };
    }

    #[test]
    fn test_connect() {
        assert_packet!(
            Packet::Connect(Connect {
                clean_session: false,
                keep_alive: 60,
                client_id: "12345",
                last_will: None,
                username: Some("user"),
                password: Some(b"pass"),
            }),
            b"\x10\x1D\x00\x04MQTT\x04\xC0\x00\x3C\x00\x0512345\x00\x04user\x00\x04pass"
        );

        assert_packet!(
            Packet::Connect(Connect {
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
            }),
            b"\x10\x21\x00\x04MQTT\x04\x14\x00\x3C\x00\x0512345\x00\x05topic\x00\x07message"
        );

        assert_packet!(Packet::Disconnect, b"\xe0\x00");
    }

    #[test]
    fn test_publish() {
        assert_packet!(
            Packet::Publish(Publish {
                dup: true,
                retain: true,
                qos: QoS::ExactlyOnce,
                topic: "topic",
                packet_id: Some(0x4321),
                payload: b"data",
            }),
            b"\x3d\x0D\x00\x05topic\x43\x21data"
        );

        assert_packet!(
            Packet::Publish(Publish {
                dup: false,
                retain: false,
                qos: QoS::AtMostOnce,
                topic: "topic",
                packet_id: None,
                payload: b"data",
            }),
            b"\x30\x0b\x00\x05topicdata"
        );
    }

    #[test]
    fn test_subscribe() {
        assert_packet!(
            Packet::Subscribe(Subscribe {
                packet_id: 0x1234,
                subscriptions: vec![("test", QoS::AtLeastOnce), ("filter", QoS::ExactlyOnce),],
            }),
            b"\x82\x12\x12\x34\x00\x04test\x01\x00\x06filter\x02"
        );

        assert_packet!(
            Packet::SubscribeAck(SubscribeAck {
                packet_id: 0x1234,
                status: vec![
                    SubscribeReturnCode::Success(QoS::AtLeastOnce),
                    SubscribeReturnCode::Failure,
                    SubscribeReturnCode::Success(QoS::ExactlyOnce),
                ],
            }),
            b"\x90\x05\x12\x34\x01\x80\x02"
        );

        assert_packet!(
            Packet::Unsubscribe(Unsubscribe {
                packet_id: 0x1234,
                topic_filters: vec!["test", "filter"],
            }),
            b"\xa2\x10\x12\x34\x00\x04test\x00\x06filter"
        );

        assert_packet!(
            Packet::UnsubscribeAck(UnsubscribeAck { packet_id: 0x4321 }),
            b"\xb0\x02\x43\x21"
        );
    }

    #[test]
    fn test_ping_pong() {
        assert_packet!(Packet::Ping, b"\xc0\x00");
        assert_packet!(Packet::Pong, b"\xd0\x00");
    }
}
//     use std::borrow::Cow;

//     use super::*;
//     use crate::decode::*;

//     #[test]
//     fn test_encode_fixed_header() {
//         let mut v = Vec::new();
//         let p = Packet::PingRequest;

//         assert_eq!(v.calc_content_size(&p), 0);
//         assert_eq!(v.write_fixed_header(&p).unwrap(), 2);
//         assert_eq!(v, b"\xc0\x00");

//         v.clear();

//         let payload = (0..255).map(|b| b).collect::<Vec<u8>>();

//         let p = Packet::Publish {
//             dup: true,
//             retain: true,
//             qos: QoS::ExactlyOnce,
//             topic: Cow::from("topic"),
//             packet_id: Some(0x4321),
//             payload: Cow::from(&payload[..]),
//         };

//         assert_eq!(v.calc_content_size(&p), 264);
//         assert_eq!(v.write_fixed_header(&p).unwrap(), 3);
//         assert_eq!(v, b"\x3d\x88\x02");
//     }

//     macro_rules! assert_packet {
//         ($p:expr, $data:expr) => {
//             let mut v = Vec::new();
//             assert_eq!(v.write_packet(&$p).unwrap(), $data.len());
//             assert_eq!(v, $data);
//             assert_eq!(read_packet($data).unwrap(), (&b""[..], $p));
//         };
//     }

// }
