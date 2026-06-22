#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
//! KNX primitives for the public KNXyz Rust crates.
//!
//! This crate currently exposes address types, shared error types, selected
//! KNXnet/IP header primitives, and `std`-gated protocol helper surfaces. It is
//! not a commissioning tool and does not perform device programming,
//! application download, Secure commissioning, or project-file write-back.
//!
//! # Feature boundaries
//!
//! - Default features enable `std`.
//! - `--no-default-features` keeps a limited `no_std` surface for addresses,
//!   errors, and selected KNXnet/IP decode primitives.
//! - `serde` adds optional derives for exported types without enabling `std`.
//! - `std` currently enables allocation-backed convenience APIs, cEMI/APCI
//!   helpers, encode helpers, and `std::error::Error` for `KnxError`.
//!
//! There is no public `alloc` feature yet. Allocation-backed APIs may move from
//! `std` to a future `alloc` feature after a separate API review and test
//! matrix update.

mod address;
#[cfg(feature = "std")]
mod apci;
#[cfg(feature = "std")]
mod cemi;
mod error;
mod knxnetip;

pub use address::{GroupAddress, IndividualAddress, TwoLevelGroupAddressDisplay};
#[cfg(feature = "std")]
pub use apci::Apci;
#[cfg(feature = "std")]
pub use cemi::{CemiFrame, CemiMessageCode, GroupTelegram};
pub use error::{KnxError, Result};
pub use knxnetip::{
    ConnectionHeader, HostProtocol, Hpai, KnxNetIpHeader, ServiceType, HEADER_LENGTH,
    PROTOCOL_VERSION_1_0,
};
