#![no_std]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

#[doc(alias = "usb")]
pub mod boot_keyboard;
#[doc(alias = "usb")]
pub mod descriptor;
#[doc(alias = "usb")]
pub mod request;

pub use descriptor::DescriptorError;

#[cfg(kani)]
mod verification;
