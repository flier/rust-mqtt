use core::mem;

use bytes::BufMut;

use crate::packet::*;

const PROPERTY_ID_SIZE: usize = mem::size_of::<u8>();
const LENGTH_FIELD_SIZE: usize = mem::size_of::<u16>();

const SUPPORTED: u8 = 1;
const UNSUPPORTED: u8 = 0;

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
            Packet::Disconnect(_) => Type::DISCONNECT,
            Packet::Auth(_) => Type::AUTH,
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
            Packet::Ping | Packet::Pong => 0,
            Packet::Disconnect(ref disconnect) => disconnect.size(),
            Packet::Auth(ref auth) => auth.size(),
        }
    }
}

trait BufMutExt: BufMut {
    fn put_utf8_str(&mut self, s: &str) {
        self.put_binary(s.as_bytes())
    }

    fn put_binary(&mut self, s: &[u8]) {
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
            Packet::Ping | Packet::Pong => {}
            Packet::Disconnect(ref disconnect) => disconnect.write_to(buf),
            Packet::Auth(ref auth) => auth.write_to(buf),
        }
    }
}

fn size_of_varint(n: usize) -> usize {
    match n {
        n if n <= 127 => 1,         // (0x7F)
        n if n <= 16_383 => 2,      // (0xFF, 0x7F)
        n if n <= 2_097_151 => 3,   // (0xFF, 0xFF, 0x7F)
        n if n <= 268_435_455 => 4, // (0xFF, 0xFF, 0xFF, 0x7F)
        _ => panic!("variable integer {} too large", n),
    }
}

impl WriteTo for FixedHeader {
    fn size(&self) -> usize {
        mem::size_of::<u8>() + size_of_varint(self.remaining_length)
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u8(((self.packet_type as u8) << 4) + self.packet_flags);
        buf.put_varint(self.remaining_length);
    }
}

impl WriteTo for [Property<'_>] {
    fn size(&self) -> usize {
        let size = self.iter().map(|prop| prop.size()).sum();

        size_of_varint(size) + size
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        if self.is_empty() {
            buf.put_u8(0);
        } else {
            buf.put_varint(self.iter().map(|prop| prop.size()).sum());

            for prop in self {
                prop.write_to(buf);
            }
        }
    }
}

impl WriteTo for Property<'_> {
    fn size(&self) -> usize {
        PROPERTY_ID_SIZE
            + match self {
                Property::PayloadFormat(_)
                | Property::RequestProblem(_)
                | Property::RequestResponse(_)
                | Property::MaximumQoS(_)
                | Property::RetainAvailable(_)
                | Property::WildcardSubscriptionAvailable(_)
                | Property::SubscriptionIdAvailable(_)
                | Property::SharedSubscriptionAvailable(_) => mem::size_of::<u8>(),

                Property::ServerKeepAlive(_)
                | Property::ReceiveMaximum(_)
                | Property::TopicAliasMaximum(_)
                | Property::TopicAlias(_) => mem::size_of::<u16>(),

                Property::MessageExpiryInterval(_)
                | Property::SessionExpiryInterval(_)
                | Property::WillDelayInterval(_)
                | Property::MaximumPacketSize(_) => mem::size_of::<u32>(),

                Property::SubscriptionId(n) => size_of_varint(*n),

                Property::CorrelationData(s) | Property::AuthData(s) => LENGTH_FIELD_SIZE + s.len(),

                Property::ContentType(s)
                | Property::ResponseTopic(s)
                | Property::AssignedClientId(s)
                | Property::AuthMethod(s)
                | Property::Response(s)
                | Property::ServerReference(s)
                | Property::Reason(s) => LENGTH_FIELD_SIZE + s.len(),

                Property::UserProperty(name, value) => {
                    LENGTH_FIELD_SIZE + name.len() + LENGTH_FIELD_SIZE + value.len()
                }
            }
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u8(self.id() as u8);

        match *self {
            Property::RetainAvailable(b)
            | Property::WildcardSubscriptionAvailable(b)
            | Property::SubscriptionIdAvailable(b)
            | Property::SharedSubscriptionAvailable(b) => {
                buf.put_u8(if b { SUPPORTED } else { UNSUPPORTED })
            }

            Property::RequestProblem(n) | Property::RequestResponse(n) => buf.put_u8(n),

            Property::PayloadFormat(n) => buf.put_u8(n as u8),
            Property::MaximumQoS(n) => buf.put_u8(n as u8),

            Property::ServerKeepAlive(n)
            | Property::ReceiveMaximum(n)
            | Property::TopicAliasMaximum(n)
            | Property::TopicAlias(n) => buf.put_u16(n),

            Property::MessageExpiryInterval(d)
            | Property::SessionExpiryInterval(d)
            | Property::WillDelayInterval(d) => buf.put_u32(d.as_secs()),
            Property::MaximumPacketSize(n) => buf.put_u32(n),

            Property::SubscriptionId(n) => buf.put_varint(n),

            Property::CorrelationData(s) | Property::AuthData(s) => buf.put_binary(s),

            Property::ContentType(s)
            | Property::ResponseTopic(s)
            | Property::AssignedClientId(s)
            | Property::AuthMethod(s)
            | Property::Response(s)
            | Property::ServerReference(s)
            | Property::Reason(s) => buf.put_utf8_str(s),

            Property::UserProperty(name, value) => {
                buf.put_utf8_str(name);
                buf.put_utf8_str(value);
            }
        }
    }
}

impl WriteTo for Connect<'_> {
    fn size(&self) -> usize {
        PROTOCOL_NAME.len()
            + mem::size_of::<u8>()                      // protocol_level
            + mem::size_of::<ConnectFlags>()            // flags
            + mem::size_of::<u16>()                     // keep_alive
            + self.properties.as_ref().map_or(0, |p| p.size())
            + LENGTH_FIELD_SIZE + self.client_id.len()  // client_id
            + self.last_will.as_ref().map_or(0, |will| {
                LENGTH_FIELD_SIZE + will.topic_name.len() + LENGTH_FIELD_SIZE + will.message.len()
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
        buf.put_u8(self.protocol_version as u8);
        buf.put_u8(flags.bits());
        buf.put_u16(self.keep_alive);
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
        buf.put_utf8_str(self.client_id);
        if let Some(ref will) = self.last_will {
            buf.put_utf8_str(will.topic_name);
            buf.put_binary(will.message);
        }
        if let Some(ref username) = self.username {
            buf.put_utf8_str(username);
        }
        if let Some(ref password) = self.password {
            buf.put_binary(password);
        }
    }
}

impl WriteTo for ConnectAck<'_> {
    fn size(&self) -> usize {
        mem::size_of::<ConnectAckFlags>()
            + mem::size_of::<ConnectReturnCode>()
            + self.properties.as_ref().map_or(0, |p| p.size())
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u8(if self.session_present {
            ConnectAckFlags::SESSION_PRESENT.bits()
        } else {
            0
        });
        buf.put_u8(self.return_code as u8);
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
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
            + self.topic_name.len()
            + self.packet_id.map_or(0, |_| mem::size_of::<PacketId>())
            + self.properties.as_ref().map_or(0, |p| p.size())
            + self.payload.len()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_utf8_str(self.topic_name);
        if let Some(packet_id) = self.packet_id {
            buf.put_u16(packet_id);
        }
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
        buf.put_slice(self.payload)
    }
}

impl WriteTo for PublishAck<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self.reason_code.map_or(0, |_| mem::size_of::<ReasonCode>())
            + self.properties.as_ref().map_or(0, |p| p.size())
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
        if let Some(reason_code) = self.reason_code {
            buf.put_u8(reason_code as u8)
        }
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
    }
}

impl WriteTo for PublishReceived<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self.reason_code.map_or(0, |_| mem::size_of::<ReasonCode>())
            + self.properties.as_ref().map_or(0, |p| p.size())
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
        if let Some(reason_code) = self.reason_code {
            buf.put_u8(reason_code as u8)
        }
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
    }
}

impl WriteTo for PublishRelease<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self.reason_code.map_or(0, |_| mem::size_of::<ReasonCode>())
            + self.properties.as_ref().map_or(0, |p| p.size())
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
        if let Some(reason_code) = self.reason_code {
            buf.put_u8(reason_code as u8)
        }
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
    }
}

impl WriteTo for PublishComplete<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self.reason_code.map_or(0, |_| mem::size_of::<ReasonCode>())
            + self.properties.as_ref().map_or(0, |p| p.size())
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
        if let Some(reason_code) = self.reason_code {
            buf.put_u8(reason_code as u8)
        }
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
    }
}

impl WriteTo for Subscribe<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self.properties.as_ref().map_or(0, |p| p.size())
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
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
        for &(topic_filter, qos) in &self.subscriptions {
            buf.put_utf8_str(topic_filter);
            buf.put_u8(qos as u8)
        }
    }
}

impl WriteTo for SubscribeAck<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self.properties.as_ref().map_or(0, |p| p.size())
            + mem::size_of::<SubscribeReturnCode>() * self.status.iter().count()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
        for &return_code in &self.status {
            buf.put_u8(return_code.into())
        }
    }
}

impl WriteTo for Unsubscribe<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self.properties.as_ref().map_or(0, |p| p.size())
            + self
                .topic_filters
                .iter()
                .map(|topic_filter| LENGTH_FIELD_SIZE + topic_filter.len())
                .sum::<usize>()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
        for &topic_filter in &self.topic_filters {
            buf.put_utf8_str(topic_filter);
        }
    }
}

impl WriteTo for UnsubscribeAck<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>() + self.properties.as_ref().map_or(0, |p| p.size())
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
    }
}

impl WriteTo for Disconnect<'_> {
    fn size(&self) -> usize {
        self.reason_code.map_or(0, |_| mem::size_of::<ReasonCode>())
            + self.properties.as_ref().map_or(0, |p| p.size())
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        if let Some(reason_code) = self.reason_code {
            buf.put_u8(reason_code as u8)
        }
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
    }
}

impl WriteTo for Auth<'_> {
    fn size(&self) -> usize {
        self.reason_code.map_or(0, |_| mem::size_of::<ReasonCode>())
            + self.properties.as_ref().map_or(0, |p| p.size())
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        if let Some(reason_code) = self.reason_code {
            buf.put_u8(reason_code as u8)
        }
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoded_data() {
        let mut v = Vec::new();

        v.put_utf8_str("hello");
        v.put_binary(b"world");
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
                protocol_version: ProtocolVersion::V311,
                clean_session: false,
                keep_alive: 60,
                properties: None,
                client_id: "12345",
                last_will: None,
                username: Some("user"),
                password: Some(b"pass"),
            }),
            b"\x10\x1D\x00\x04MQTT\x04\xC0\x00\x3C\x00\x0512345\x00\x04user\x00\x04pass"
        );

        assert_packet!(
            Packet::Connect(Connect {
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
            }),
            b"\x10\x21\x00\x04MQTT\x04\x14\x00\x3C\x00\x0512345\x00\x05topic\x00\x07message"
        );

        assert_packet!(
            Packet::Connect(Connect {
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
            }),
            b"\x10\x2C\x00\x04MQTT\x05\x14\x00\x3C\x0A\x11\xff\xff\xff\xff\x27\x00\x00\x10\x00\x00\x0512345\x00\x05topic\x00\x07message"
        );

        assert_packet!(
            Packet::Disconnect(Disconnect {
                reason_code: None,
                properties: None
            }),
            b"\xe0\x00"
        );
    }

    #[test]
    fn test_publish() {
        assert_packet!(
            Packet::Publish(Publish {
                dup: true,
                retain: true,
                qos: QoS::ExactlyOnce,
                topic_name: "topic",
                packet_id: Some(0x4321),
                properties: None,
                payload: b"data",
            }),
            b"\x3d\x0D\x00\x05topic\x43\x21data"
        );

        assert_packet!(
            Packet::Publish(Publish {
                dup: false,
                retain: false,
                qos: QoS::AtMostOnce,
                topic_name: "topic",
                packet_id: None,
                properties: None,
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
                properties: None,
                subscriptions: vec![("test", QoS::AtLeastOnce), ("filter", QoS::ExactlyOnce),],
            }),
            b"\x82\x12\x12\x34\x00\x04test\x01\x00\x06filter\x02"
        );

        assert_packet!(
            Packet::SubscribeAck(SubscribeAck {
                packet_id: 0x1234,
                properties: None,
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
                properties: None,
                topic_filters: vec!["test", "filter"],
            }),
            b"\xa2\x10\x12\x34\x00\x04test\x00\x06filter"
        );

        assert_packet!(
            Packet::UnsubscribeAck(UnsubscribeAck {
                packet_id: 0x4321,
                properties: None,
            }),
            b"\xb0\x02\x43\x21"
        );
    }

    #[test]
    fn test_ping_pong() {
        assert_packet!(Packet::Ping, b"\xc0\x00");
        assert_packet!(Packet::Pong, b"\xd0\x00");
    }
}
