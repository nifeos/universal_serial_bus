//! **Setup packets: what a host asks a device** (USB 2.0 section 9.3, section 9.4; HID 1.11 section 7.2).
//!
//! Every control transfer opens with eight bytes saying who the request is for, what it is, and
//! how much data follows in which direction. A host controller carries them as the immediate data
//! of a setup-stage transfer request block (TRB), which is why [`SetupPacket::to_u64`] exists: the packet is one little-
//! endian quadword there.

/// `bmRequestType` bit 7: data flows device to host.
pub const DEVICE_TO_HOST: u8 = 0x80;
/// `bmRequestType` bits 6:5 = 1: a class request (here, HID's).
pub const CLASS: u8 = 0x20;
/// `bmRequestType` bits 4:0 = 1: addressed to an interface.
pub const TO_INTERFACE: u8 = 0x01;

/// `GET_DESCRIPTOR` (USB 2.0 Table 9-4).
pub const GET_DESCRIPTOR: u8 = 6;
/// `SET_CONFIGURATION` (USB 2.0 Table 9-4).
pub const SET_CONFIGURATION: u8 = 9;
/// HID `SET_IDLE` (HID 1.11 section 7.2.4).
pub const SET_IDLE: u8 = 0x0a;
/// HID `SET_PROTOCOL` (HID 1.11 section 7.2.6).
pub const SET_PROTOCOL: u8 = 0x0b;
/// `SET_PROTOCOL`'s value for the boot protocol (report protocol is 1).
pub const BOOT_PROTOCOL: u16 = 0;

/// Which way a request's data stage flows: bit 7 of `bmRequestType` (USB 2.0 Table 9-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Host to device, bit 7 clear; also every request with no data stage.
    HostToDevice,
    /// Device to host, bit 7 set.
    DeviceToHost,
}

/// Who defines a request: bits 6:5 of `bmRequestType` (USB 2.0 Table 9-2). The value 3 is
/// reserved, so no variant names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum RequestKind {
    /// A standard request of USB 2.0 section 9.4, such as `GET_DESCRIPTOR`.
    Standard,
    /// A request a device class defines, such as HID's `SET_IDLE`.
    Class,
    /// A request the vendor defines.
    Vendor,
}

/// What a request is addressed to: bits 4:0 of `bmRequestType` (USB 2.0 Table 9-2). The values 4
/// to 31 are reserved, so no variant names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Recipient {
    /// The device as a whole.
    Device,
    /// One interface, named in `wIndex`.
    Interface,
    /// One endpoint, named in `wIndex`.
    Endpoint,
    /// Something else the class or vendor defines.
    Other,
}

/// **`bmRequestType`**: a request's direction, kind and recipient packed in one byte. Built from
/// the three parts, or parsed from a byte that holds no reserved value, so every value is one the
/// specification defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(alias = "bmRequestType")]
pub struct RequestType(u8);

impl RequestType {
    /// The byte that packs `direction`, `kind` and `recipient`.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::{Direction, Recipient, RequestKind, RequestType};
    ///
    /// let to_interface = RequestType::new(Direction::HostToDevice, RequestKind::Class, Recipient::Interface);
    /// assert_eq!(to_interface.get(), 0x21);
    /// ```
    pub const fn new(direction: Direction, kind: RequestKind, recipient: Recipient) -> RequestType {
        let direction = match direction {
            Direction::HostToDevice => 0,
            Direction::DeviceToHost => DEVICE_TO_HOST,
        };
        let kind = match kind {
            RequestKind::Standard => 0,
            RequestKind::Class => CLASS,
            RequestKind::Vendor => 0x40,
        };
        let recipient = match recipient {
            Recipient::Device => 0,
            Recipient::Interface => TO_INTERFACE,
            Recipient::Endpoint => 2,
            Recipient::Other => 3,
        };
        RequestType(direction | kind | recipient)
    }

    /// Read `bmRequestType` from the byte a setup packet carries. `None` for a reserved kind (3)
    /// or a reserved recipient (4 to 31).
    ///
    /// # Errors
    ///
    /// `None` when the kind bits hold the reserved value 3 or the recipient bits a reserved value.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::{Direction, Recipient, RequestType};
    ///
    /// # fn main() {
    /// #     assert!(example().is_some());
    /// # }
    /// # fn example() -> Option<()> {
    /// let request_type = RequestType::parse(0x81)?;
    /// assert_eq!(request_type.direction(), Direction::DeviceToHost);
    /// assert_eq!(request_type.recipient(), Recipient::Interface);
    /// assert_eq!(RequestType::parse(0x60), None); // kind 3 is reserved
    /// # Some(())
    /// # }
    /// ```
    pub const fn parse(byte: u8) -> Option<RequestType> {
        if (byte >> 5) & 0x3 == 3 || byte & 0x1f > 3 {
            return None;
        }
        Some(RequestType(byte))
    }

    /// The byte itself, as the first byte of the packet.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::SetupPacket;
    ///
    /// assert_eq!(SetupPacket::set_boot_protocol(0).request_type.get(), 0x21);
    /// ```
    pub const fn get(self) -> u8 {
        self.0
    }

    /// Which way the data stage flows.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::{Direction, SetupPacket};
    ///
    /// assert_eq!(SetupPacket::get_descriptor(1, 0, 18).request_type.direction(), Direction::DeviceToHost);
    /// ```
    pub const fn direction(self) -> Direction {
        if self.0 & DEVICE_TO_HOST != 0 {
            Direction::DeviceToHost
        } else {
            Direction::HostToDevice
        }
    }

    /// Who defines the request.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::{RequestKind, SetupPacket};
    ///
    /// assert_eq!(SetupPacket::set_idle_forever(0).request_type.kind(), RequestKind::Class);
    /// ```
    pub const fn kind(self) -> RequestKind {
        match (self.0 >> 5) & 0x3 {
            0 => RequestKind::Standard,
            1 => RequestKind::Class,
            _ => RequestKind::Vendor,
        }
    }

    /// What the request is addressed to.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::{Recipient, SetupPacket};
    ///
    /// assert_eq!(SetupPacket::set_configuration(1).request_type.recipient(), Recipient::Device);
    /// ```
    pub const fn recipient(self) -> Recipient {
        match self.0 & 0x1f {
            0 => Recipient::Device,
            1 => Recipient::Interface,
            2 => Recipient::Endpoint,
            _ => Recipient::Other,
        }
    }
}

/// How many bytes a setup packet holds (USB 2.0 Table 9-2).
pub const SETUP_PACKET_BYTES: usize = 8;

/// **One setup packet.** Field names are the specification's, without the Hungarian prefixes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupPacket {
    /// `bmRequestType`: direction, kind and recipient.
    pub request_type: RequestType,
    /// `bRequest`.
    pub request: u8,
    /// `wValue`.
    pub value: u16,
    /// `wIndex`.
    pub index: u16,
    /// (`length`, `wLength` in USB 2.0 Table 9-2): how many bytes the data stage carries, zero for
    /// none.
    #[doc(alias = "wLength")]
    pub length: u16,
}

impl SetupPacket {
    /// `GET_DESCRIPTOR` of `kind` (a [`crate::descriptor`] type) number `index`, at most `length`
    /// bytes. A device may answer fewer, and the host must accept a short answer. The parameter
    /// `index` is the descriptor index, which goes in the low byte of `wValue`; the packet's own
    /// [`SetupPacket::index`] field is `wIndex`, which this request leaves zero.
    ///
    /// # Examples
    ///
    /// The first request a host sends, for the first eight bytes of the device descriptor:
    ///
    /// ```
    /// use universal_serial_bus::descriptor::DEVICE;
    /// use universal_serial_bus::request::SetupPacket;
    ///
    /// let packet = SetupPacket::get_descriptor(DEVICE, 0, 8);
    /// assert_eq!(packet.to_le_bytes(), [0x80, 6, 0, 1, 0, 0, 8, 0]);
    /// ```
    pub const fn get_descriptor(kind: u8, index: u8, length: u16) -> SetupPacket {
        SetupPacket {
            request_type: RequestType::new(
                Direction::DeviceToHost,
                RequestKind::Standard,
                Recipient::Device,
            ),
            request: GET_DESCRIPTOR,
            value: (kind as u16) << 8 | index as u16,
            index: 0,
            length,
        }
    }

    /// `SET_CONFIGURATION` to the configuration whose `bConfigurationValue` is `value`.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::SetupPacket;
    ///
    /// assert_eq!(SetupPacket::set_configuration(1).to_le_bytes(), [0, 9, 1, 0, 0, 0, 0, 0]);
    /// ```
    pub const fn set_configuration(value: u8) -> SetupPacket {
        SetupPacket {
            request_type: RequestType::new(
                Direction::HostToDevice,
                RequestKind::Standard,
                Recipient::Device,
            ),
            request: SET_CONFIGURATION,
            value: value as u16,
            index: 0,
            length: 0,
        }
    }

    /// HID `SET_PROTOCOL` to the boot protocol, on `interface`. HID 1.11 section 7.2.6 says a host should
    /// send it rather than assume the protocol, because every device starts in report protocol; some
    /// keyboards stall it and report boot anyway, which is why the driver tolerates a stall here.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::SetupPacket;
    ///
    /// assert_eq!(SetupPacket::set_boot_protocol(0).to_le_bytes(), [0x21, 0x0b, 0, 0, 0, 0, 0, 0]);
    /// ```
    pub const fn set_boot_protocol(interface: u8) -> SetupPacket {
        SetupPacket {
            request_type: RequestType::new(
                Direction::HostToDevice,
                RequestKind::Class,
                Recipient::Interface,
            ),
            request: SET_PROTOCOL,
            value: BOOT_PROTOCOL,
            index: interface as u16,
            length: 0,
        }
    }

    /// HID `SET_IDLE` with a duration of zero on `interface`: report only when something changes.
    /// What a boot keyboard defaults to in practice, asked for because a device that repeated
    /// its last report would look like a key held down to nothing at all.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::SetupPacket;
    ///
    /// assert_eq!(SetupPacket::set_idle_forever(1).to_le_bytes(), [0x21, 0x0a, 0, 0, 1, 0, 0, 0]);
    /// ```
    pub const fn set_idle_forever(interface: u8) -> SetupPacket {
        SetupPacket {
            request_type: RequestType::new(
                Direction::HostToDevice,
                RequestKind::Class,
                Recipient::Interface,
            ),
            request: SET_IDLE,
            value: 0,
            index: interface as u16,
            length: 0,
        }
    }

    /// Whether a data stage, if there is one, flows device to host.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::SetupPacket;
    ///
    /// assert!(SetupPacket::get_descriptor(2, 0, 9).is_device_to_host());
    /// assert!(!SetupPacket::set_configuration(1).is_device_to_host());
    /// ```
    pub const fn is_device_to_host(&self) -> bool {
        matches!(self.request_type.direction(), Direction::DeviceToHost)
    }

    /// **The eight bytes as one little-endian quadword**, the order they go on the wire and the
    /// layout an xHCI setup-stage TRB carries as immediate data.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::SetupPacket;
    ///
    /// assert_eq!(SetupPacket::set_configuration(1).to_u64(), 0x0000_0000_0001_0900);
    /// ```
    pub const fn to_u64(&self) -> u64 {
        u64::from_le_bytes(self.to_le_bytes())
    }

    /// **The eight bytes in the order they go on the wire**, each multi-byte field little-endian,
    /// as a host controller that takes the setup stage as bytes writes them (the buffer of a
    /// driver for a controller without the setup-stage block of the Extensible Host Controller
    /// Interface).
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::SetupPacket;
    ///
    /// let bytes = SetupPacket::get_descriptor(2, 0, 34).to_le_bytes();
    /// assert_eq!(bytes, [0x80, 6, 0, 2, 0, 0, 34, 0]);
    /// ```
    pub const fn to_le_bytes(&self) -> [u8; SETUP_PACKET_BYTES] {
        let value = self.value.to_le_bytes();
        let index = self.index.to_le_bytes();
        let length = self.length.to_le_bytes();
        [
            self.request_type.get(),
            self.request,
            value[0],
            value[1],
            index[0],
            index[1],
            length[0],
            length[1],
        ]
    }

    /// **Read a setup packet from the bytes on the wire**, as a test or a device-side reader does.
    /// `None` for fewer than [`SETUP_PACKET_BYTES`] bytes, and for a `bmRequestType` that holds a
    /// reserved value ([`RequestType::parse`]). A longer buffer is accepted and its tail ignored.
    ///
    /// # Errors
    ///
    /// `None` when `bytes` holds fewer than [`SETUP_PACKET_BYTES`] bytes, and when `bmRequestType`
    /// holds a reserved value.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::request::SetupPacket;
    ///
    /// # fn main() {
    /// #     assert!(example().is_some());
    /// # }
    /// # fn example() -> Option<()> {
    /// let packet = SetupPacket::parse(&[0x21, 0x0b, 0, 0, 1, 0, 0, 0])?;
    /// assert_eq!(packet, SetupPacket::set_boot_protocol(1));
    /// assert_eq!(SetupPacket::parse(&[0x21, 0x0b]), None);
    /// # Some(())
    /// # }
    /// ```
    pub const fn parse(bytes: &[u8]) -> Option<SetupPacket> {
        if bytes.len() < SETUP_PACKET_BYTES {
            return None;
        }
        let Some(request_type) = RequestType::parse(bytes[0]) else {
            return None;
        };
        Some(SetupPacket {
            request_type,
            request: bytes[1],
            value: u16::from_le_bytes([bytes[2], bytes[3]]),
            index: u16::from_le_bytes([bytes[4], bytes[5]]),
            length: u16::from_le_bytes([bytes[6], bytes[7]]),
        })
    }
}

impl From<SetupPacket> for [u8; SETUP_PACKET_BYTES] {
    /// The packet's bytes on the wire, as [`SetupPacket::to_le_bytes`] gives them.
    fn from(packet: SetupPacket) -> [u8; SETUP_PACKET_BYTES] {
        packet.to_le_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The device descriptor's first eight bytes, the first request any host sends: `80 06 00 01
    /// 00 00 08 00` on the wire.
    #[test]
    fn get_descriptor_is_the_bytes_the_wire_carries() {
        let p = SetupPacket::get_descriptor(crate::descriptor::DEVICE, 0, 8);
        assert_eq!(
            p.to_u64().to_le_bytes(),
            [0x80, 0x06, 0x00, 0x01, 0x00, 0x00, 0x08, 0x00]
        );
        assert!(p.is_device_to_host());
    }

    #[test]
    fn the_class_requests_are_sent_to_the_interface() {
        assert_eq!(
            SetupPacket::set_boot_protocol(2).to_u64().to_le_bytes(),
            [0x21, 0x0b, 0, 0, 2, 0, 0, 0]
        );
        assert_eq!(
            SetupPacket::set_idle_forever(0).to_u64().to_le_bytes(),
            [0x21, 0x0a, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            SetupPacket::set_configuration(1).to_u64().to_le_bytes(),
            [0x00, 0x09, 1, 0, 0, 0, 0, 0]
        );
        assert!(!SetupPacket::set_configuration(1).is_device_to_host());
    }

    /// The three parts pack where USB 2.0 Table 9-2 puts them, and read back as themselves.
    #[test]
    fn a_request_type_packs_its_three_parts() {
        let all = [
            (
                Direction::DeviceToHost,
                RequestKind::Vendor,
                Recipient::Other,
                0xc3,
            ),
            (
                Direction::HostToDevice,
                RequestKind::Class,
                Recipient::Endpoint,
                0x22,
            ),
            (
                Direction::HostToDevice,
                RequestKind::Standard,
                Recipient::Device,
                0x00,
            ),
            (
                Direction::DeviceToHost,
                RequestKind::Class,
                Recipient::Interface,
                0xa1,
            ),
        ];
        for (direction, kind, recipient, byte) in all {
            let request_type = RequestType::new(direction, kind, recipient);
            assert_eq!(request_type.get(), byte);
            assert_eq!(RequestType::parse(byte), Some(request_type));
            assert_eq!(request_type.direction(), direction);
            assert_eq!(request_type.kind(), kind);
            assert_eq!(request_type.recipient(), recipient);
        }
    }

    /// A reserved kind or recipient is refused, and every other byte reads back as itself.
    #[test]
    fn a_reserved_request_type_is_refused() {
        for byte in 0..=u8::MAX {
            let reserved = (byte >> 5) & 0x3 == 3 || byte & 0x1f > 3;
            assert_eq!(
                RequestType::parse(byte).map(RequestType::get),
                (!reserved).then_some(byte),
                "byte {byte:#x}"
            );
        }
    }

    /// The bytes and the parse are inverses, and a short buffer or a reserved request type is
    /// refused.
    #[test]
    fn a_packet_reads_back_from_its_bytes() {
        let packet = SetupPacket::get_descriptor(crate::descriptor::CONFIGURATION, 0, 0x1234);
        let bytes: [u8; SETUP_PACKET_BYTES] = packet.into();
        assert_eq!(bytes, [0x80, 0x06, 0x00, 0x02, 0x00, 0x00, 0x34, 0x12]);
        assert_eq!(SetupPacket::parse(&bytes), Some(packet));
        assert_eq!(packet.to_u64(), u64::from_le_bytes(bytes));
        let interface = SetupPacket {
            index: 0x0102,
            value: 0x0304,
            ..SetupPacket::set_idle_forever(0)
        };
        assert_eq!(interface.to_le_bytes()[2..6], [0x04, 0x03, 0x02, 0x01]);
        assert_eq!(
            SetupPacket::parse(&interface.to_le_bytes()),
            Some(interface)
        );
        assert_eq!(SetupPacket::parse(&bytes[..7]), None);
        let mut reserved = bytes;
        reserved[0] = 0x60;
        assert_eq!(SetupPacket::parse(&reserved), None);
    }
}
