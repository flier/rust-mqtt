use core::mem;

use bytes::BufMut;
use derive_more::Deref;

use crate::{
    mqtt::{ConnectReturnCode, PacketId, Property, ReasonCode},
    packet::*,
};

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
            Packet::Publish(ref publish) => Publish(publish).flags().bits(),
            Packet::PublishRelease(_) | Packet::Subscribe(_) | Packet::Unsubscribe(_) => 0x02,
            _ => 0,
        }
    }

    fn remaining_length(&self) -> usize {
        match self {
            Packet::Connect(ref connect) => Connect(connect).size(),
            Packet::ConnectAck(ref connect_ack) => ConnectAck(connect_ack).size(),
            Packet::Publish(ref publish) => Publish(publish).size(),
            Packet::PublishAck(ref publish_ack) => PublishAck(publish_ack).size(),
            Packet::PublishReceived(ref publish_received) => {
                PublishReceived(publish_received).size()
            }
            Packet::PublishRelease(ref publish_release) => PublishRelease(publish_release).size(),
            Packet::PublishComplete(ref publish_complete) => {
                PublishComplete(publish_complete).size()
            }
            Packet::Subscribe(ref subscribe) => Subscribe(subscribe).size(),
            Packet::SubscribeAck(ref subscribe_ack) => SubscribeAck(subscribe_ack).size(),
            Packet::Unsubscribe(ref unsubscribe) => Unsubscribe(unsubscribe).size(),
            Packet::UnsubscribeAck(ref unsubscribe_ack) => UnsubscribeAck(unsubscribe_ack).size(),
            Packet::Ping | Packet::Pong => 0,
            Packet::Disconnect(ref disconnect) => Disconnect(disconnect).size(),
            Packet::Auth(ref auth) => Auth(auth).size(),
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
            Packet::Connect(ref connect) => Connect(connect).write_to(buf),
            Packet::ConnectAck(ref connect_ack) => ConnectAck(connect_ack).write_to(buf),
            Packet::Publish(ref publish) => Publish(publish).write_to(buf),
            Packet::PublishAck(ref publish_ack) => PublishAck(publish_ack).write_to(buf),
            Packet::PublishReceived(ref publish_received) => {
                PublishReceived(publish_received).write_to(buf)
            }
            Packet::PublishRelease(ref publish_release) => {
                PublishRelease(publish_release).write_to(buf)
            }
            Packet::PublishComplete(ref publish_complete) => {
                PublishComplete(publish_complete).write_to(buf)
            }
            Packet::Subscribe(ref subscribe) => Subscribe(subscribe).write_to(buf),
            Packet::SubscribeAck(ref subscribe_ack) => SubscribeAck(subscribe_ack).write_to(buf),
            Packet::Unsubscribe(ref unsubscribe) => Unsubscribe(unsubscribe).write_to(buf),
            Packet::UnsubscribeAck(ref unsubscribe_ack) => {
                UnsubscribeAck(unsubscribe_ack).write_to(buf)
            }
            Packet::Ping | Packet::Pong => {}
            Packet::Disconnect(ref disconnect) => Disconnect(disconnect).write_to(buf),
            Packet::Auth(ref auth) => Auth(auth).write_to(buf),
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
                | Property::RequestProblemInformation(_)
                | Property::RequestResponseInformation(_)
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

                Property::SubscriptionId(n) => size_of_varint(*n as usize),

                Property::CorrelationData(s) | Property::AuthData(s) => LENGTH_FIELD_SIZE + s.len(),

                Property::ContentType(s)
                | Property::ResponseTopic(s)
                | Property::AssignedClientId(s)
                | Property::AuthMethod(s)
                | Property::ResponseInformation(s)
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
            | Property::SharedSubscriptionAvailable(b)
            | Property::RequestProblemInformation(b)
            | Property::RequestResponseInformation(b) => {
                buf.put_u8(if b { SUPPORTED } else { UNSUPPORTED })
            }

            Property::PayloadFormat(n) => buf.put_u8(n as u8),
            Property::MaximumQoS(n) => buf.put_u8(n as u8),

            Property::ReceiveMaximum(n)
            | Property::TopicAliasMaximum(n)
            | Property::TopicAlias(n) => buf.put_u16(n),

            Property::ServerKeepAlive(d) => buf.put_u16(d.as_secs() as u16),

            Property::MessageExpiryInterval(d) | Property::WillDelayInterval(d) => {
                buf.put_u32(d.as_secs() as u32)
            }
            Property::SessionExpiryInterval(d) => buf.put_u32(d.as_secs()),
            Property::MaximumPacketSize(n) => buf.put_u32(n),

            Property::SubscriptionId(n) => buf.put_varint(n as usize),

            Property::CorrelationData(s) | Property::AuthData(s) => buf.put_binary(s),

            Property::ContentType(s)
            | Property::ResponseTopic(s)
            | Property::AssignedClientId(s)
            | Property::AuthMethod(s)
            | Property::ResponseInformation(s)
            | Property::ServerReference(s)
            | Property::Reason(s) => buf.put_utf8_str(s),

            Property::UserProperty(name, value) => {
                buf.put_utf8_str(name);
                buf.put_utf8_str(value);
            }
        }
    }
}

#[derive(Deref)]
struct Connect<'a>(&'a mqtt::Connect<'a>);

impl WriteTo for Connect<'_> {
    fn size(&self) -> usize {
        PROTOCOL_NAME.len()
            + mem::size_of::<u8>()                      // protocol_level
            + mem::size_of::<ConnectFlags>()            // flags
            + mem::size_of::<u16>()                     // keep_alive
            + self.properties.as_ref().map_or(0, |p| p.size())
            + LENGTH_FIELD_SIZE + self.client_id.len()  // client_id
            + self.last_will.as_ref().map_or(0, |will| {
                will.properties.as_ref().map_or(0, |p| p.size())
                + LENGTH_FIELD_SIZE + will.topic_name.len()
                + LENGTH_FIELD_SIZE + will.message.len()
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
            if let Some(ref props) = will.properties {
                props.write_to(buf);
            }
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

#[derive(Deref)]
struct ConnectAck<'a>(&'a mqtt::ConnectAck<'a>);

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

#[derive(Deref)]
struct Publish<'a>(&'a mqtt::Publish<'a>);

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

#[derive(Deref)]
struct PublishAck<'a>(&'a mqtt::PublishAck<'a>);

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

#[derive(Deref)]
struct PublishReceived<'a>(&'a mqtt::PublishReceived<'a>);

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

#[derive(Deref)]
struct PublishRelease<'a>(&'a mqtt::PublishRelease<'a>);

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

#[derive(Deref)]
struct PublishComplete<'a>(&'a mqtt::PublishComplete<'a>);

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

#[derive(Deref)]
struct Subscribe<'a>(&'a mqtt::Subscribe<'a>);

impl WriteTo for Subscribe<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self.properties.as_ref().map_or(0, |p| p.size())
            + self
                .subscriptions
                .iter()
                .map(|subscription| {
                    LENGTH_FIELD_SIZE
                        + subscription.topic_filter.len()
                        + mem::size_of::<SubscriptionOptions>()
                })
                .sum::<usize>()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
        for subscription in &self.subscriptions {
            buf.put_utf8_str(subscription.topic_filter);
            buf.put_u8(SubscriptionOptions::from(subscription).bits())
        }
    }
}

#[derive(Deref)]
struct SubscribeAck<'a>(&'a mqtt::SubscribeAck<'a>);

impl WriteTo for SubscribeAck<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self.properties.as_ref().map_or(0, |p| p.size())
            + mem::size_of::<u8>() * self.status.iter().count()
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
        for &return_code in &self.status {
            buf.put_u8(match return_code {
                Ok(qos) => qos as u8,
                Err(reason) => reason as u8,
            })
        }
    }
}

#[derive(Deref)]
struct Unsubscribe<'a>(&'a mqtt::Unsubscribe<'a>);

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

#[derive(Deref)]
struct UnsubscribeAck<'a>(&'a mqtt::UnsubscribeAck<'a>);

impl WriteTo for UnsubscribeAck<'_> {
    fn size(&self) -> usize {
        mem::size_of::<PacketId>()
            + self.properties.as_ref().map_or(0, |p| p.size())
            + self.status.as_ref().map_or(0, |props| props.len())
    }

    fn write_to<T: BufMut>(&self, buf: &mut T) {
        buf.put_u16(self.packet_id);
        if let Some(ref props) = self.properties {
            props.write_to(buf);
        }
        if let Some(ref status) = self.status {
            for code in status {
                buf.put_u8(match *code {
                    Ok(_) => RETURN_CODE_SUCCESS,
                    Err(reason) => reason as u8,
                })
            }
        }
    }
}

#[derive(Deref)]
struct Disconnect<'a>(&'a mqtt::Disconnect<'a>);

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

#[derive(Deref)]
struct Auth<'a>(&'a mqtt::Auth<'a>);

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
    use crate::mqtt::*;

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
            Packet::Connect(mqtt::Connect {
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
            Packet::Connect(mqtt::Connect {
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
                    properties: None,
                }),
                username: None,
                password: None,
            }),
            b"\x10\x21\x00\x04MQTT\x04\x14\x00\x3C\x00\x0512345\x00\x05topic\x00\x07message"
        );

        assert_packet!(
            Packet::Connect(mqtt::Connect {
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
                    properties: None,
                }),
                username: None,
                password: None,
            }),
            b"\x10\x2C\x00\x04MQTT\x05\x14\x00\x3C\x0A\x11\xff\xff\xff\xff\x27\x00\x00\x10\x00\x00\x0512345\x00\x05topic\x00\x07message"
        );

        assert_packet!(
            Packet::Disconnect(mqtt::Disconnect {
                reason_code: None,
                properties: None
            }),
            b"\xe0\x00"
        );
    }

    #[test]
    fn test_publish() {
        assert_packet!(
            Packet::Publish(mqtt::Publish {
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
            Packet::Publish(mqtt::Publish {
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
            Packet::Subscribe(mqtt::Subscribe {
                packet_id: 0x1234,
                properties: None,
                subscriptions: vec![
                    Subscription {
                        topic_filter: "test",
                        qos: QoS::AtLeastOnce,
                        ..Default::default()
                    },
                    Subscription {
                        topic_filter: "filter",
                        qos: QoS::ExactlyOnce,
                        ..Default::default()
                    }
                ],
            }),
            b"\x82\x12\x12\x34\x00\x04test\x01\x00\x06filter\x02"
        );

        assert_packet!(
            Packet::SubscribeAck(mqtt::SubscribeAck {
                packet_id: 0x1234,
                properties: None,
                status: vec![
                    Ok(QoS::AtLeastOnce),
                    Err(ReasonCode::UnspecifiedError),
                    Ok(QoS::ExactlyOnce),
                ],
            }),
            b"\x90\x05\x12\x34\x01\x80\x02"
        );

        assert_packet!(
            Packet::Unsubscribe(mqtt::Unsubscribe {
                packet_id: 0x1234,
                properties: None,
                topic_filters: vec!["test", "filter"],
            }),
            b"\xa2\x10\x12\x34\x00\x04test\x00\x06filter"
        );

        assert_packet!(
            Packet::UnsubscribeAck(mqtt::UnsubscribeAck {
                packet_id: 0x4321,
                properties: None,
                status: None,
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
