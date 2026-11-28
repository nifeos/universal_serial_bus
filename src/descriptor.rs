//! **Descriptors: what a device says about itself** (USB 2.0 section 9.6, HID 1.11 section 6.2).
//!
//! A descriptor is a run of bytes whose first byte is its own length and whose second is its type.
//! A configuration descriptor is followed, in one transfer, by every interface and endpoint
//! descriptor beneath it, concatenated, so reading one is a walk over a linked structure whose
//! links are lengths **the device chose**. That is the whole hazard: a zero length loops forever,
//! a length past the end reads past the end, and a walk that trusts either is a driver a cheap
//! keyboard can crash. [`Descriptors`] checks both before every step, and [`find_boot_keyboard`]
//! walks with it.

/// `bDescriptorType` of a device descriptor.
pub const DEVICE: u8 = 1;
/// `bDescriptorType` of a configuration descriptor.
pub const CONFIGURATION: u8 = 2;
/// `bDescriptorType` of an interface descriptor.
pub const INTERFACE: u8 = 4;
/// `bDescriptorType` of an endpoint descriptor.
pub const ENDPOINT: u8 = 5;

/// How many bytes a device descriptor holds (USB 2.0 Table 9-8).
pub const DEVICE_DESCRIPTOR_BYTES: usize = 18;
/// How many bytes a configuration descriptor's own header holds, before its interfaces (Table
/// 9-10). Not the configuration's whole length, which is its `wTotalLength`.
pub const CONFIGURATION_HEADER_BYTES: usize = 9;
/// How many bytes an interface descriptor holds (Table 9-12).
pub const INTERFACE_DESCRIPTOR_BYTES: usize = 9;
/// How many bytes an endpoint descriptor holds (Table 9-13). USB 3 appends a companion descriptor rather
/// than lengthening this one, so 7 holds at every speed.
pub const ENDPOINT_DESCRIPTOR_BYTES: usize = 7;

/// The HID interface class (HID 1.11 section 4.1).
pub const CLASS_HID: u8 = 3;
/// The HID boot interface subclass (HID 1.11 section 4.2): the device promises the fixed boot report.
pub const SUBCLASS_BOOT: u8 = 1;
/// The boot keyboard protocol (HID 1.11 section 4.3).
pub const PROTOCOL_KEYBOARD: u8 = 1;

/// **Why a descriptor was refused.** Each names the fact a person at the bench needs, because "the
/// keyboard does not work" is the report this driver would otherwise produce for all of them.
///
/// A later release may add a refusal, or a field to one, so a `match` on it ends in a wildcard
/// arm and a pattern on a variant's fields ends in `..`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum DescriptorError {
    /// Fewer bytes than the descriptor's fixed header needs.
    Truncated,
    /// The bytes are not the descriptor they were asked for: the wrong type, or a length field
    /// shorter than that type's fixed layout.
    NotThatDescriptor,
    /// A descriptor inside a configuration has a length of zero or one, runs past the end of
    /// what the device sent, or is an interface or endpoint descriptor too short for its layout.
    /// `offset` is its byte offset, for the bench transcript.
    #[non_exhaustive]
    Malformed {
        /// Byte offset of the bad descriptor within the configuration.
        offset: u16,
    },
    /// The configuration has no interface of class 3, subclass 1, protocol 1. The device is not a
    /// keyboard, or is one that does not offer the boot protocol.
    NoBootKeyboard,
    /// A boot keyboard interface with no interrupt `IN` endpoint, which no conforming keyboard sends.
    #[non_exhaustive]
    NoInterruptIn {
        /// The interface that had none.
        interface: u8,
    },
    /// The interrupt endpoint's packets are smaller than one eight-byte boot report, so a report
    /// would arrive in pieces this driver does not reassemble.
    #[non_exhaustive]
    PacketTooSmall {
        /// What the endpoint descriptor declared.
        maximum_packet_bytes: u16,
    },
}

/// One line in lower case with no closing period, naming the field a variant carries, so a caller
/// can print the refusal on its own or after its own context.
impl core::fmt::Display for DescriptorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Truncated => f.write_str("descriptor shorter than its fixed header"),
            Self::NotThatDescriptor => f.write_str("not the descriptor that was asked for"),
            Self::Malformed { offset } => {
                write!(
                    f,
                    "malformed descriptor at byte offset {offset} of the configuration"
                )
            }
            Self::NoBootKeyboard => f.write_str("no boot keyboard interface in the configuration"),
            Self::NoInterruptIn { interface } => write!(
                f,
                "boot keyboard interface {interface} has no interrupt in endpoint"
            ),
            Self::PacketTooSmall {
                maximum_packet_bytes,
            } => write!(
                f,
                "interrupt endpoint packets of {maximum_packet_bytes} bytes cannot hold an \
                 eight-byte boot report"
            ),
        }
    }
}

/// A refusal wraps no other error, so `source` is `None`.
impl core::error::Error for DescriptorError {}

/// **The device descriptor**, the fields a driver uses from it. Only [`DeviceDescriptor::parse`]
/// builds one, and a later release may add a field, so a pattern on it ends in `..`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DeviceDescriptor {
    /// `bcdUSB`: the USB revision the device claims, in binary-coded decimal (0x0200 for 2.0).
    #[doc(alias = "bcdUSB")]
    pub usb_release_binary_coded_decimal: u16,
    /// `bDeviceClass`. Zero means "each interface says for itself", which is what keyboards send.
    pub class: u8,
    /// `bDeviceSubClass`, read with [`DeviceDescriptor::class`]: class 0xef with subclass 2 and
    /// protocol 1 says the configuration groups its interfaces with interface associations.
    pub subclass: u8,
    /// `bDeviceProtocol`, read with [`DeviceDescriptor::class`] and [`DeviceDescriptor::subclass`].
    pub protocol: u8,
    /// (`control_packet_size_code`, `bMaxPacketSize0` in USB 2.0 Table 9-8): the control endpoint's
    /// packet size in bytes below `SuperSpeed`, and the exponent of it at `SuperSpeed` and above, so
    /// a code to interpret and carrying no unit. [`control_maximum_packet_bytes`] interprets it.
    #[doc(alias = "bMaxPacketSize0")]
    pub control_packet_size_code: u8,
    /// `idVendor`.
    pub vendor: u16,
    /// `idProduct`.
    pub product: u16,
    /// `bcdDevice`: the device's release number, in binary-coded decimal, as the vendor chose it.
    #[doc(alias = "bcdDevice")]
    pub device_release_binary_coded_decimal: u16,
    /// `iManufacturer`: the index of the string descriptor naming the manufacturer, zero for none.
    #[doc(alias = "iManufacturer")]
    pub manufacturer_string_index: u8,
    /// `iProduct`: the index of the string descriptor naming the product, zero for none.
    #[doc(alias = "iProduct")]
    pub product_string_index: u8,
    /// `iSerialNumber`: the index of the string descriptor with the serial number, zero for none.
    #[doc(alias = "iSerialNumber")]
    pub serial_number_string_index: u8,
    /// `bNumConfigurations`.
    pub configurations: u8,
}

impl DeviceDescriptor {
    /// Parse a device descriptor. [`DescriptorError::Truncated`] for fewer than
    /// [`DEVICE_DESCRIPTOR_BYTES`] bytes, and [`DescriptorError::NotThatDescriptor`] for the wrong
    /// type or a `bLength` shorter than the layout.
    ///
    /// # Errors
    ///
    /// [`DescriptorError::Truncated`] when `bytes` holds fewer than [`DEVICE_DESCRIPTOR_BYTES`]
    /// bytes, and [`DescriptorError::NotThatDescriptor`] when `bDescriptorType` is not [`DEVICE`]
    /// or `bLength` is shorter than the layout.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::DescriptorError;
    /// use universal_serial_bus::descriptor::DeviceDescriptor;
    ///
    /// fn main() -> Result<(), DescriptorError> {
    ///     // A USB 2.0 device, vendor 0x0627, product 0x0001, a 64-byte control endpoint.
    ///     let bytes = [18, 1, 0, 2, 0, 0, 0, 64, 0x27, 0x06, 1, 0, 0, 0, 1, 2, 3, 1];
    ///     let device = DeviceDescriptor::parse(&bytes)?;
    ///     assert_eq!((device.vendor, device.product), (0x0627, 0x0001));
    ///     assert_eq!(DeviceDescriptor::parse(&bytes[..8]), Err(DescriptorError::Truncated));
    ///     Ok(())
    /// }
    /// ```
    pub fn parse(bytes: &[u8]) -> Result<DeviceDescriptor, DescriptorError> {
        if bytes.len() < DEVICE_DESCRIPTOR_BYTES {
            return Err(DescriptorError::Truncated);
        }
        if (bytes[0] as usize) < DEVICE_DESCRIPTOR_BYTES || bytes[1] != DEVICE {
            return Err(DescriptorError::NotThatDescriptor);
        }
        Ok(DeviceDescriptor {
            usb_release_binary_coded_decimal: u16::from_le_bytes([bytes[2], bytes[3]]),
            class: bytes[4],
            subclass: bytes[5],
            protocol: bytes[6],
            control_packet_size_code: bytes[7],
            vendor: u16::from_le_bytes([bytes[8], bytes[9]]),
            product: u16::from_le_bytes([bytes[10], bytes[11]]),
            device_release_binary_coded_decimal: u16::from_le_bytes([bytes[12], bytes[13]]),
            manufacturer_string_index: bytes[14],
            product_string_index: bytes[15],
            serial_number_string_index: bytes[16],
            configurations: bytes[17],
        })
    }
}

/// **The speed a device runs at**, in the USB specification's terms. It is USB vocabulary and not
/// a host controller's numbering: a host controller driver maps its own port speed onto it. A
/// later release may name another speed, so a `match` on it ends in a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Speed {
    /// Low speed, 1.5 Mb/s (USB 2.0).
    Low,
    /// Full speed, 12 Mb/s (USB 2.0).
    Full,
    /// High speed, 480 Mb/s (USB 2.0).
    High,
    /// `SuperSpeed` or faster (USB 3.2), every speed at which `bMaxPacketSize0` is an exponent.
    Super,
}

/// **The control endpoint's packet size in bytes**, from `control_packet_size_code`, the
/// `bMaxPacketSize0` the device sent in the first eight bytes of its device descriptor. `None` for
/// a code the specification does not allow at `speed`.
///
/// Below `SuperSpeed` the code is the size (USB 2.0 section 9.6.1), and section 5.5.3 fixes which
/// sizes each speed allows: 8 at low speed, 8, 16, 32 or 64 at full speed, and 64 at high speed.
/// At `SuperSpeed` it is an exponent and must be 9, for 512 bytes (USB 3.2 section 9.6.1).
/// Refusing anything else matters because the host controller is told this number and sizes every
/// control transfer by it. Linux's hub driver also refuses a low-speed device that does not declare
/// 8, and accepts 8 to 64 at high speed with a warning (`drivers/usb/core/hub.c` at
/// `torvalds/linux@23c62304dde29abe92013d85910d03a77dada35d`); this function refuses that, as the
/// specification does.
///
/// # Errors
///
/// `None` when `control_packet_size_code` is not a size (or, at `SuperSpeed`, an exponent) the
/// specification allows at `speed`.
///
/// # Examples
///
/// ```
/// use universal_serial_bus::descriptor::{Speed, control_maximum_packet_bytes};
///
/// assert_eq!(control_maximum_packet_bytes(64, Speed::Full), Some(64));
/// assert_eq!(control_maximum_packet_bytes(9, Speed::Super), Some(512));
/// assert_eq!(control_maximum_packet_bytes(16, Speed::Low), None); // low speed allows only 8
/// ```
pub fn control_maximum_packet_bytes(control_packet_size_code: u8, speed: Speed) -> Option<u16> {
    match (speed, control_packet_size_code) {
        (Speed::Low, 8) | (Speed::Full, 8 | 16 | 32 | 64) | (Speed::High, 64) => {
            Some(u16::from(control_packet_size_code))
        }
        (Speed::Super, 9) => Some(512),
        _ => None,
    }
}

/// **The configuration descriptor's header**: how long the whole configuration is, and the value
/// `SET_CONFIGURATION` selects it with. Only [`ConfigurationHeader::parse`] builds one, and a later
/// release may add a field, so a pattern on it ends in `..`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ConfigurationHeader {
    /// (`total_length`, `wTotalLength` in USB 2.0 Table 9-10): the bytes of the header plus every
    /// descriptor beneath it.
    #[doc(alias = "wTotalLength")]
    pub total_length: u16,
    /// `bNumInterfaces`.
    pub interfaces: u8,
    /// `bConfigurationValue`, the argument to `SET_CONFIGURATION`.
    pub value: u8,
    /// `bmAttributes`: bit 6 set for a device that powers itself in this configuration, bit 5 set
    /// for one that supports remote wakeup, bit 7 set as the specification requires.
    #[doc(alias = "bmAttributes")]
    pub attributes: u8,
    /// (`maximum_power_code`, `bMaxPower` in USB 2.0 Table 9-10): the most current the device draws
    /// from the bus in this configuration, in units of 2 milliamperes below `SuperSpeed` and of 8
    /// milliamperes at `SuperSpeed` and above (USB 3.2), so a code carrying no unit.
    #[doc(alias = "bMaxPower")]
    pub maximum_power_code: u8,
}

impl ConfigurationHeader {
    /// Parse the first [`CONFIGURATION_HEADER_BYTES`] bytes of a configuration descriptor.
    ///
    /// # Errors
    ///
    /// [`DescriptorError::Truncated`] when `bytes` holds fewer than [`CONFIGURATION_HEADER_BYTES`]
    /// bytes, and [`DescriptorError::NotThatDescriptor`] when `bDescriptorType` is not
    /// [`CONFIGURATION`] or `bLength` is shorter than the header.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::DescriptorError;
    /// use universal_serial_bus::descriptor::ConfigurationHeader;
    ///
    /// fn main() -> Result<(), DescriptorError> {
    ///     let header = ConfigurationHeader::parse(&[9, 2, 34, 0, 1, 1, 0, 0xa0, 50])?;
    ///     assert_eq!((header.total_length, header.interfaces, header.value), (34, 1, 1));
    ///     Ok(())
    /// }
    /// ```
    pub fn parse(bytes: &[u8]) -> Result<ConfigurationHeader, DescriptorError> {
        if bytes.len() < CONFIGURATION_HEADER_BYTES {
            return Err(DescriptorError::Truncated);
        }
        if (bytes[0] as usize) < CONFIGURATION_HEADER_BYTES || bytes[1] != CONFIGURATION {
            return Err(DescriptorError::NotThatDescriptor);
        }
        Ok(ConfigurationHeader {
            total_length: u16::from_le_bytes([bytes[2], bytes[3]]),
            interfaces: bytes[4],
            value: bytes[5],
            attributes: bytes[7],
            maximum_power_code: bytes[8],
        })
    }
}

/// **A keyboard that speaks the boot protocol, and where its reports arrive.** Everything the
/// driver needs to configure the device and the host controller's endpoint, and nothing else.
/// Only [`find_boot_keyboard`] builds one, so every value describes an endpoint a device sent, and
/// a later release may add a field, so a pattern on it ends in `..`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct BootKeyboard {
    /// The `bConfigurationValue` to select with `SET_CONFIGURATION`.
    pub configuration: u8,
    /// The interface number, for the class requests `SET_PROTOCOL` and `SET_IDLE`.
    pub interface: u8,
    /// The interrupt `IN` endpoint's number and direction (`0x81` for endpoint 1 `IN`). The
    /// specification's reserved bits 6:4 of `bEndpointAddress` are cleared.
    pub endpoint: u8,
    /// The endpoint's packet size in bytes (`wMaxPacketSize` bits 10:0); at least 8, or the walk
    /// refused it.
    pub maximum_packet_bytes: u16,
    /// `bInterval` as the device sent it. What it means depends on the speed, which is the host
    /// controller driver's business.
    pub interval: u8,
}

/// **The descriptors beneath a configuration header, one at a time**: each interface, endpoint,
/// class and vendor descriptor's bytes, length and type included, in the order the device sent
/// them. A caller looking for a mouse, a hub or a second interface walks with it, and
/// [`find_boot_keyboard`] is one such walk.
///
/// The walk begins after the configuration header's own `bLength`, so a header longer than nine
/// bytes skips what the device says it carries. It stops at the lesser of `wTotalLength` and the
/// buffer's length, so a read cut short is a shorter walk rather than an overrun. Every descriptor
/// it yields holds at least its own two bytes of length and type, and an interface or endpoint
/// descriptor holds at least its fixed layout. A descriptor that breaks that (a length of 0 or 1,
/// one that runs past `wTotalLength`, or an interface or endpoint too short for its layout) is
/// yielded as [`DescriptorError::Malformed`] with its offset, and the walk ends there.
#[derive(Debug, Clone)]
pub struct Descriptors<'a> {
    configuration: &'a [u8],
    header: ConfigurationHeader,
    /// The offset of the next descriptor.
    at: usize,
    /// Set once the walk has ended, after a refusal or at the end of the bytes.
    done: bool,
}

impl<'a> Descriptors<'a> {
    /// Begin a walk over `configuration`, the configuration descriptor as the device sent it,
    /// possibly cut short by the buffer it was read into.
    ///
    /// # Errors
    ///
    /// What [`ConfigurationHeader::parse`] refuses: [`DescriptorError::Truncated`] for fewer than
    /// [`CONFIGURATION_HEADER_BYTES`] bytes, and [`DescriptorError::NotThatDescriptor`] for bytes
    /// that are not a configuration header. A descriptor beneath the header that is malformed is
    /// not refused here; the walk yields [`DescriptorError::Malformed`] when it reaches it.
    ///
    /// # Examples
    ///
    /// A caller looking for something other than a keyboard, here every endpoint address:
    ///
    /// ```
    /// use universal_serial_bus::DescriptorError;
    /// use universal_serial_bus::descriptor::{Descriptors, ENDPOINT};
    ///
    /// fn main() -> Result<(), DescriptorError> {
    ///     let configuration = [
    ///         9, 2, 25, 0, 1, 1, 0, 0xa0, 50, // configuration 1, 25 bytes
    ///         9, 4, 0, 0, 1, 3, 1, 2, 0, // interface 0: HID, boot, mouse
    ///         7, 5, 0x81, 3, 4, 0, 10, // endpoint 1 `IN`, interrupt, 4 bytes
    ///     ];
    ///     let mut endpoints = Vec::new();
    ///     for descriptor in Descriptors::new(&configuration)? {
    ///         let descriptor = descriptor?;
    ///         if descriptor[1] == ENDPOINT {
    ///             endpoints.push(descriptor[2]);
    ///         }
    ///     }
    ///     assert_eq!(endpoints, [0x81]);
    ///     Ok(())
    /// }
    /// ```
    pub fn new(configuration: &'a [u8]) -> Result<Descriptors<'a>, DescriptorError> {
        let header = ConfigurationHeader::parse(configuration)?;
        Ok(Descriptors {
            configuration,
            header,
            at: usize::from(configuration[0]),
            done: false,
        })
    }

    /// The configuration's header, which the walk read first.
    ///
    /// # Examples
    ///
    /// ```
    /// use universal_serial_bus::DescriptorError;
    /// use universal_serial_bus::descriptor::Descriptors;
    ///
    /// fn main() -> Result<(), DescriptorError> {
    ///     let walk = Descriptors::new(&[9, 2, 9, 0, 0, 1, 0, 0x80, 50])?;
    ///     assert_eq!(walk.header().value, 1);
    ///     assert_eq!(walk.count(), 0); // a header with nothing beneath it
    ///     Ok(())
    /// }
    /// ```
    pub fn header(&self) -> ConfigurationHeader {
        self.header
    }

    /// End the walk with a refusal naming the descriptor at `at`.
    fn malformed(&mut self, at: usize) -> Option<Result<&'a [u8], DescriptorError>> {
        self.done = true;
        Some(Err(DescriptorError::Malformed {
            offset: u16::try_from(at).unwrap_or(u16::MAX),
        }))
    }
}

impl<'a> Iterator for Descriptors<'a> {
    type Item = Result<&'a [u8], DescriptorError>;

    fn next(&mut self) -> Option<Result<&'a [u8], DescriptorError>> {
        let configuration = self.configuration;
        let total = usize::from(self.header.total_length);
        let end = configuration.len().min(total);
        let at = self.at;
        if self.done || at + 2 > end {
            self.done = true;
            return None;
        }
        let len = usize::from(configuration[at]);
        if len < 2 || at + len > total {
            return self.malformed(at);
        }
        if at + len > configuration.len() {
            // Well formed by the device's own total, and cut off by the buffer it was read into.
            self.done = true;
            return None;
        }
        // A descriptor of a type a walk reads by its layout, too short for that layout, is
        // refused: skipping it would leave the previous interface open, and the endpoint that
        // follows would be read as that interface's.
        let layout = match configuration[at + 1] {
            INTERFACE => INTERFACE_DESCRIPTOR_BYTES,
            ENDPOINT => ENDPOINT_DESCRIPTOR_BYTES,
            _ => 2,
        };
        if len < layout {
            return self.malformed(at);
        }
        self.at = at + len;
        Some(Ok(&configuration[at..at + len]))
    }
}

impl core::iter::FusedIterator for Descriptors<'_> {}

/// **Find the boot keyboard in a configuration**: the first interface (alternate setting 0) of
/// class 3, subclass 1, protocol 1, and the first interrupt `IN` endpoint beneath it.
///
/// The walk over `configuration`, possibly cut short, is [`Descriptors`]'. It answers at the first
/// keyboard endpoint, so a later defect, common in cheap devices, is never read.
///
/// # Errors
///
/// What [`Descriptors`] refuses: [`DescriptorError::Truncated`] or
/// [`DescriptorError::NotThatDescriptor`] for the header, and [`DescriptorError::Malformed`] for a
/// descriptor the walk reaches before a keyboard endpoint. Then, when the walk ends with no
/// keyboard endpoint: [`DescriptorError::PacketTooSmall`] when a boot keyboard's interrupt `IN`
/// endpoint was refused for its packet size, [`DescriptorError::NoInterruptIn`] when a boot
/// keyboard interface had no interrupt `IN` endpoint, and [`DescriptorError::NoBootKeyboard`] when
/// no interface was a boot keyboard.
///
/// # Examples
///
/// A boot keyboard's configuration as QEMU's `usb-kbd` reports it, trimmed to what matters:
///
/// ```
/// use universal_serial_bus::DescriptorError;
/// use universal_serial_bus::descriptor::find_boot_keyboard;
///
/// # fn main() -> Result<(), DescriptorError> {
/// let configuration = [
///     9, 2, 34, 0, 1, 1, 0, 0xa0, 50, // configuration 1, 34 bytes, one interface
///     9, 4, 0, 0, 1, 3, 1, 1, 0, // interface 0: HID, boot, keyboard, one endpoint
///     9, 0x21, 0x11, 1, 0, 1, 0x22, 63, 0, // the HID descriptor, which the walk skips
///     7, 5, 0x81, 3, 8, 0, 10, // endpoint 1 `IN`, interrupt, 8 bytes, every 10 frames
/// ];
/// let keyboard = find_boot_keyboard(&configuration)?;
/// assert_eq!(keyboard.configuration, 1);
/// assert_eq!(keyboard.interface, 0);
/// assert_eq!(keyboard.endpoint, 0x81);
/// assert_eq!(keyboard.maximum_packet_bytes, 8);
/// assert_eq!(keyboard.interval, 10);
/// # Ok(())
/// # }
/// ```
pub fn find_boot_keyboard(configuration: &[u8]) -> Result<BootKeyboard, DescriptorError> {
    let descriptors = Descriptors::new(configuration)?;
    let header = descriptors.header();

    // The interface currently being read, when it is a boot keyboard: its number. `None` while
    // the walk is inside some other interface.
    let mut candidate: Option<u8> = None;
    // The last boot keyboard interface that ended with no interrupt IN, for the refusal.
    let mut without_endpoint: Option<u8> = None;
    // An endpoint refused for its packet size, for the refusal, if nothing better turns up.
    let mut too_small: Option<u16> = None;

    for d in descriptors {
        let d = d?;
        match d[1] {
            INTERFACE => {
                if let Some(interface) = candidate {
                    without_endpoint = Some(interface);
                }
                let is_boot_keyboard = d[3] == 0 // alternate setting 0, the one SET_CONFIGURATION selects
                    && d[5] == CLASS_HID
                    && d[6] == SUBCLASS_BOOT
                    && d[7] == PROTOCOL_KEYBOARD;
                candidate = is_boot_keyboard.then_some(d[2]);
            }
            ENDPOINT => {
                if let Some(interface) = candidate {
                    let endpoint_number_and_direction = d[2] & 0x8f;
                    let interrupt = d[3] & 0x3 == 0x3;
                    let inbound = endpoint_number_and_direction & 0x80 != 0;
                    let number = endpoint_number_and_direction & 0x0f;
                    let maximum_packet_bytes = u16::from_le_bytes([d[4], d[5]]) & 0x07ff;
                    if interrupt && inbound && number != 0 {
                        if maximum_packet_bytes < crate::boot_keyboard::REPORT_BYTES as u16 {
                            too_small = Some(maximum_packet_bytes);
                        } else {
                            return Ok(BootKeyboard {
                                configuration: header.value,
                                interface,
                                endpoint: endpoint_number_and_direction,
                                maximum_packet_bytes,
                                interval: d[6],
                            });
                        }
                    }
                }
            }
            // The HID class descriptor, string and vendor descriptors, interface associations,
            // `SuperSpeed` companions: none of them changes which endpoint the reports arrive on.
            _ => {}
        }
    }
    if let Some(maximum_packet_bytes) = too_small {
        return Err(DescriptorError::PacketTooSmall {
            maximum_packet_bytes,
        });
    }
    match candidate.or(without_endpoint) {
        Some(interface) => Err(DescriptorError::NoInterruptIn { interface }),
        None => Err(DescriptorError::NoBootKeyboard),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each refusal prints one lower-case line with its field, and is a `core::error::Error`.
    #[test]
    fn each_refusal_prints_its_own_line_and_is_an_error() {
        extern crate std;
        use std::string::ToString;
        let cases = [
            (
                DescriptorError::Truncated,
                "descriptor shorter than its fixed header",
            ),
            (
                DescriptorError::NotThatDescriptor,
                "not the descriptor that was asked for",
            ),
            (
                DescriptorError::Malformed { offset: 18 },
                "malformed descriptor at byte offset 18 of the configuration",
            ),
            (
                DescriptorError::NoBootKeyboard,
                "no boot keyboard interface in the configuration",
            ),
            (
                DescriptorError::NoInterruptIn { interface: 2 },
                "boot keyboard interface 2 has no interrupt in endpoint",
            ),
            (
                DescriptorError::PacketTooSmall {
                    maximum_packet_bytes: 4,
                },
                "interrupt endpoint packets of 4 bytes cannot hold an eight-byte boot report",
            ),
        ];
        for (refusal, message) in cases {
            assert_eq!(refusal.to_string(), message);
            let error: &dyn core::error::Error = &refusal;
            assert!(error.source().is_none());
        }
    }

    /// QEMU's `usb-kbd` configuration, byte for byte as `hw/usb/dev-hid.c` builds it, which is the
    /// device the end-to-end gate types on.
    const QEMU_USB_KEYBOARD: [u8; 34] = [
        9, 2, 34, 0, 1, 1, 6, 0xa0, 50, // configuration
        9, 4, 0, 0, 1, 3, 1, 1, 0, // interface 0, HID boot keyboard
        9, 0x21, 0x11, 1, 0, 1, 0x22, 63, 0, // HID descriptor
        7, 5, 0x81, 3, 8, 0, 7, // endpoint 1 IN, interrupt, 8 bytes, bInterval 7
    ];

    #[test]
    fn qemus_keyboard_is_found() {
        assert_eq!(
            find_boot_keyboard(&QEMU_USB_KEYBOARD),
            Ok(BootKeyboard {
                configuration: 1,
                interface: 0,
                endpoint: 0x81,
                maximum_packet_bytes: 8,
                interval: 7,
            })
        );
    }

    /// **A composite keyboard**, the common real shape: interface 0 is the boot keyboard, interface
    /// 1 is a media-key HID interface with no boot subclass, each with its own endpoint. The walk
    /// must take interface 0's endpoint and not the last one it saw.
    #[test]
    fn a_composite_keyboard_answers_its_boot_interface() {
        let configuration = [
            9, 2, 59, 0, 2, 1, 0, 0xa0, 50, // configuration, two interfaces
            9, 4, 0, 0, 1, 3, 1, 1, 0, // interface 0: boot keyboard
            9, 0x21, 0x11, 1, 0, 1, 0x22, 65, 0, //
            7, 5, 0x81, 3, 8, 0, 10, // endpoint 1 IN
            9, 4, 1, 0, 1, 3, 0, 0, 0, // interface 1: HID, no boot subclass
            9, 0x21, 0x11, 1, 0, 1, 0x22, 50, 0, //
            7, 5, 0x82, 3, 16, 0, 10, // endpoint 2 IN
        ];
        let k = find_boot_keyboard(&configuration).unwrap();
        assert_eq!((k.interface, k.endpoint), (0, 0x81));
    }

    /// The boot interface second, behind a mouse: the walk keeps looking.
    #[test]
    fn a_keyboard_behind_another_interface_is_found() {
        let configuration = [
            9, 2, 41, 0, 2, 1, 0, 0xa0, 50, //
            9, 4, 0, 0, 1, 3, 1, 2, 0, // interface 0: boot MOUSE (protocol 2)
            7, 5, 0x81, 3, 4, 0, 10, //
            9, 4, 1, 0, 1, 3, 1, 1, 0, // interface 1: boot keyboard
            7, 5, 0x82, 3, 8, 0, 10, //
        ];
        let k = find_boot_keyboard(&configuration).unwrap();
        assert_eq!((k.interface, k.endpoint), (1, 0x82));
    }

    /// A mass-storage stick is not a keyboard, and says so rather than failing somewhere later.
    #[test]
    fn a_storage_device_is_refused_as_not_a_keyboard() {
        let configuration = [
            9, 2, 32, 0, 1, 1, 0, 0x80, 50, //
            9, 4, 0, 0, 2, 8, 6, 0x50, 0, // mass storage, SCSI, bulk-only
            7, 5, 0x81, 2, 0, 2, 0, // bulk IN
            7, 5, 0x02, 2, 0, 2, 0, // bulk OUT
        ];
        assert_eq!(
            find_boot_keyboard(&configuration),
            Err(DescriptorError::NoBootKeyboard)
        );
    }

    /// An endpoint in the direction `OUT`, host to device, or a bulk one, under a keyboard
    /// interface is not where reports arrive.
    #[test]
    fn only_an_interrupt_in_endpoint_counts() {
        let configuration = [
            9, 2, 32, 0, 1, 1, 0, 0xa0, 50, //
            9, 4, 0, 0, 2, 3, 1, 1, 0, //
            7, 5, 0x01, 3, 8, 0, 10, // interrupt OUT (the LED endpoint some keyboards have)
            7, 5, 0x82, 2, 8, 0, 10, // bulk IN
        ];
        assert_eq!(
            find_boot_keyboard(&configuration),
            Err(DescriptorError::NoInterruptIn { interface: 0 })
        );
    }

    /// Endpoint number 0 is the control endpoint, which no endpoint descriptor may name: an
    /// interrupt `IN` descriptor claiming it is not where reports arrive.
    #[test]
    fn an_endpoint_numbered_zero_is_not_the_report_endpoint() {
        let mut configuration = QEMU_USB_KEYBOARD;
        configuration[29] = 0x80; // bEndpointAddress: IN, number 0
        assert_eq!(
            find_boot_keyboard(&configuration),
            Err(DescriptorError::NoInterruptIn { interface: 0 })
        );
    }

    /// **A zero length is the infinite loop**, and a length past the end is the overrun. Both stop
    /// the walk and name the offset.
    #[test]
    fn a_zero_or_overlong_length_is_malformed_and_never_loops() {
        let mut configuration = QEMU_USB_KEYBOARD;
        configuration[9] = 0; // the interface descriptor's bLength
        assert_eq!(
            find_boot_keyboard(&configuration),
            Err(DescriptorError::Malformed { offset: 9 })
        );
        let mut configuration = QEMU_USB_KEYBOARD;
        configuration[27] = 200; // the endpoint descriptor claims to run far past the end
        assert_eq!(
            find_boot_keyboard(&configuration),
            Err(DescriptorError::Malformed { offset: 27 })
        );
    }

    /// A defect after the keyboard was found does not lose the keyboard.
    #[test]
    fn a_trailing_defect_after_the_keyboard_is_tolerated() {
        let mut configuration = [0u8; 36];
        configuration[..34].copy_from_slice(&QEMU_USB_KEYBOARD);
        configuration[2] = 36; // wTotalLength covers the junk
        configuration[34] = 0; // a zero-length descriptor
        assert!(find_boot_keyboard(&configuration).is_ok());
    }

    /// `wTotalLength` larger than what was read is a truncated read, which is a shorter walk.
    #[test]
    fn a_truncated_read_is_a_shorter_walk_not_an_overrun() {
        let mut configuration = QEMU_USB_KEYBOARD;
        configuration[2] = 0xff;
        configuration[3] = 0xff;
        assert!(find_boot_keyboard(&configuration).is_ok());
        assert_eq!(
            find_boot_keyboard(&configuration[..20]),
            Err(DescriptorError::NoInterruptIn { interface: 0 })
        );
        assert_eq!(
            find_boot_keyboard(&configuration[..4]),
            Err(DescriptorError::Truncated)
        );
    }

    #[test]
    fn a_device_descriptor_parses_and_a_short_one_does_not() {
        // QEMU usb-kbd's device descriptor.
        let d = [
            18, 1, 0x00, 0x02, 0, 0, 0, 8, 0x27, 0x06, 0x01, 0x00, 0, 0, 1, 4, 11, 1,
        ];
        let device = DeviceDescriptor::parse(&d).unwrap();
        assert_eq!(device.usb_release_binary_coded_decimal, 0x0200);
        assert_eq!(device.control_packet_size_code, 8);
        assert_eq!((device.vendor, device.product), (0x0627, 0x0001));
        assert_eq!(device.configurations, 1);
        assert_eq!((device.class, device.subclass, device.protocol), (0, 0, 0));
        assert_eq!(device.device_release_binary_coded_decimal, 0x0000);
        assert_eq!(
            (
                device.manufacturer_string_index,
                device.product_string_index,
                device.serial_number_string_index
            ),
            (1, 4, 11)
        );
        assert_eq!(
            DeviceDescriptor::parse(&d[..8]),
            Err(DescriptorError::Truncated)
        );
        let mut wrong = d;
        wrong[1] = CONFIGURATION;
        assert_eq!(
            DeviceDescriptor::parse(&wrong),
            Err(DescriptorError::NotThatDescriptor)
        );
    }

    /// A device descriptor whose `bLength` is shorter than the layout is refused, however many
    /// bytes the buffer holds; a longer one (a future revision's) is read.
    #[test]
    fn a_device_descriptor_reads_its_own_length() {
        let mut d = [
            18, 1, 0x00, 0x02, 0, 0, 0, 8, 0x27, 0x06, 0x01, 0x00, 0, 0, 1, 4, 11, 1,
        ];
        d[0] = 17;
        assert_eq!(
            DeviceDescriptor::parse(&d),
            Err(DescriptorError::NotThatDescriptor)
        );
        d[0] = 20;
        assert!(DeviceDescriptor::parse(&d).is_ok());
    }

    /// Exactly the nine header bytes parse; a short `bLength` or a different type is not this
    /// descriptor.
    #[test]
    fn a_configuration_header_is_nine_bytes_of_type_two() {
        let header = ConfigurationHeader::parse(&QEMU_USB_KEYBOARD[..9]).unwrap();
        assert_eq!((header.total_length, header.value), (34, 1));
        assert_eq!((header.attributes, header.maximum_power_code), (0xa0, 50));
        let mut short = QEMU_USB_KEYBOARD;
        short[0] = 8;
        assert_eq!(
            ConfigurationHeader::parse(&short),
            Err(DescriptorError::NotThatDescriptor)
        );
        let mut long = QEMU_USB_KEYBOARD;
        long[0] = 10;
        assert!(ConfigurationHeader::parse(&long).is_ok());
        let mut other = QEMU_USB_KEYBOARD;
        other[1] = INTERFACE;
        assert_eq!(
            ConfigurationHeader::parse(&other),
            Err(DescriptorError::NotThatDescriptor)
        );
    }

    /// A boot mouse whose endpoint is big enough for a keyboard report is still a mouse: every
    /// one of class, subclass and protocol has to match.
    #[test]
    fn a_boot_mouse_with_a_wide_endpoint_is_passed_over() {
        let configuration = [
            9, 2, 41, 0, 2, 1, 0, 0xa0, 50, //
            9, 4, 0, 0, 1, 3, 1, 2, 0, // interface 0: boot mouse
            7, 5, 0x81, 3, 8, 0, 10, // 8 bytes, as wide as a keyboard report
            9, 4, 1, 0, 1, 3, 1, 1, 0, // interface 1: boot keyboard
            7, 5, 0x82, 3, 8, 0, 10, //
        ];
        let k = find_boot_keyboard(&configuration).unwrap();
        assert_eq!((k.interface, k.endpoint), (1, 0x82));
    }

    /// A descriptor of type interface or endpoint too short for its layout is refused with its
    /// offset: skipping it would leave the previous interface open and hand the next interface's
    /// endpoint to it. A two-byte vendor descriptor is well formed and skipped.
    #[test]
    fn a_short_interface_or_endpoint_is_malformed_and_two_bytes_is_enough() {
        let configuration = [
            9, 2, 35, 0, 1, 1, 0, 0xa0, 50, //
            2, 0xff, // a vendor descriptor of length and type alone
            9, 4, 0, 0, 1, 3, 1, 1, 0, // the boot keyboard
            7, 5, 0x82, 3, 8, 0, 10, //
        ];
        let k = find_boot_keyboard(&configuration).unwrap();
        assert_eq!((k.interface, k.endpoint), (0, 0x82));

        let short_interface = [
            9, 2, 39, 0, 2, 1, 0, 0xa0, 50, //
            9, 4, 0, 0, 0, 3, 1, 1, 0, // boot keyboard with no endpoint
            4, 4, 1, 0, // an interface cut short
            7, 5, 0x82, 3, 8, 0, 10, //
        ];
        assert_eq!(
            find_boot_keyboard(&short_interface),
            Err(DescriptorError::Malformed { offset: 18 })
        );
        let short_endpoint = [
            9, 2, 29, 0, 1, 1, 0, 0xa0, 50, //
            9, 4, 0, 0, 1, 3, 1, 1, 0, //
            4, 5, 0x81, 3, // an endpoint cut short
        ];
        assert_eq!(
            find_boot_keyboard(&short_endpoint),
            Err(DescriptorError::Malformed { offset: 18 })
        );
    }

    /// A boot interface with no endpoint, then a non-boot HID interface with one: the walk
    /// refuses with the boot interface's number, and does not take the other's endpoint.
    #[test]
    fn a_boot_interface_without_an_endpoint_is_refused_by_number() {
        let configuration = [
            9, 2, 41, 0, 2, 1, 0, 0xa0, 50, //
            9, 4, 0, 0, 0, 3, 1, 1, 0, // interface 0: boot keyboard, zero endpoints
            9, 4, 1, 0, 1, 3, 0, 0, 0, // interface 1: HID, not boot
            7, 5, 0x82, 3, 8, 0, 10, //
        ];
        assert_eq!(
            find_boot_keyboard(&configuration),
            Err(DescriptorError::NoInterruptIn { interface: 0 })
        );
    }

    /// The walk yields each descriptor whole, in order, then ends; a caller can look for any
    /// interface with it.
    #[test]
    fn the_descriptors_are_walked_whole_and_in_order() {
        let mut walk = Descriptors::new(&QEMU_USB_KEYBOARD).unwrap();
        assert_eq!(walk.header().value, 1);
        assert_eq!(walk.next(), Some(Ok(&QEMU_USB_KEYBOARD[9..18])));
        assert_eq!(walk.next(), Some(Ok(&QEMU_USB_KEYBOARD[18..27])));
        assert_eq!(walk.next(), Some(Ok(&QEMU_USB_KEYBOARD[27..34])));
        assert_eq!(walk.next(), None);
        assert_eq!(walk.next(), None);
        assert_eq!(
            Descriptors::new(&QEMU_USB_KEYBOARD[..4]).err(),
            Some(DescriptorError::Truncated)
        );
    }

    /// A refusal names the offset and ends the walk, and a read cut short ends it quietly.
    #[test]
    fn a_malformed_descriptor_ends_the_walk_with_its_offset() {
        let mut configuration = QEMU_USB_KEYBOARD;
        configuration[18] = 1;
        let mut walk = Descriptors::new(&configuration).unwrap();
        assert!(matches!(walk.next(), Some(Ok(_))));
        assert_eq!(
            walk.next(),
            Some(Err(DescriptorError::Malformed { offset: 18 }))
        );
        assert_eq!(walk.next(), None);
        let cut = Descriptors::new(&QEMU_USB_KEYBOARD[..30]).unwrap();
        assert_eq!(cut.count(), 2);
    }

    /// A two-byte descriptor that ends exactly at `wTotalLength` is still yielded, and a
    /// descriptor's layout is read from its type byte, not its length byte: a three-byte endpoint
    /// descriptor is refused even though a type of 3 would need only two bytes.
    #[test]
    fn the_walk_reads_to_the_last_byte_and_checks_the_layout_by_type() {
        let header = [9, 2, 11, 0, 1, 1, 0, 0xa0, 50];
        let mut configuration = [0u8; 11];
        configuration[..9].copy_from_slice(&header);
        configuration[9..].copy_from_slice(&[2, 0x30]);
        let mut walk = Descriptors::new(&configuration).unwrap();
        assert_eq!(walk.next(), Some(Ok(&configuration[9..11])));
        assert_eq!(walk.next(), None);

        let mut configuration = [0u8; 12];
        configuration[..9].copy_from_slice(&header);
        configuration[2] = 12;
        configuration[9..].copy_from_slice(&[3, ENDPOINT, 0x81]);
        let mut walk = Descriptors::new(&configuration).unwrap();
        assert_eq!(
            walk.next(),
            Some(Err(DescriptorError::Malformed { offset: 9 }))
        );
    }

    /// The reserved bits of `bEndpointAddress` do not reach the caller.
    #[test]
    fn the_endpoint_carries_only_its_number_and_direction() {
        let mut configuration = QEMU_USB_KEYBOARD;
        configuration[29] = 0xf1;
        assert_eq!(find_boot_keyboard(&configuration).unwrap().endpoint, 0x81);
    }

    /// Each speed allows only its own sizes: a low-speed keyboard that declares 64, or a
    /// high-speed one that declares 8, is refused.
    #[test]
    fn the_control_packet_size_is_checked_against_the_speed() {
        assert_eq!(control_maximum_packet_bytes(8, Speed::Low), Some(8));
        assert_eq!(control_maximum_packet_bytes(64, Speed::Low), None);
        for size in [8, 16, 32, 64] {
            assert_eq!(
                control_maximum_packet_bytes(size, Speed::Full),
                Some(u16::from(size))
            );
        }
        assert_eq!(control_maximum_packet_bytes(64, Speed::High), Some(64));
        assert_eq!(control_maximum_packet_bytes(8, Speed::High), None);
        assert_eq!(control_maximum_packet_bytes(9, Speed::Super), Some(512));
        assert_eq!(control_maximum_packet_bytes(9, Speed::Full), None);
        assert_eq!(control_maximum_packet_bytes(0, Speed::Full), None);
        assert_eq!(control_maximum_packet_bytes(64, Speed::Super), None);
    }

    /// A keyboard whose endpoint cannot carry one report in a packet is refused by name.
    #[test]
    fn an_endpoint_smaller_than_a_report_is_refused() {
        let mut configuration = QEMU_USB_KEYBOARD;
        configuration[31] = 4; // wMaxPacketSize
        assert_eq!(
            find_boot_keyboard(&configuration),
            Err(DescriptorError::PacketTooSmall {
                maximum_packet_bytes: 4
            })
        );
    }
}
