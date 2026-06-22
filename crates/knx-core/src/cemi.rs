use core::convert::TryFrom;

use crate::{Apci, GroupAddress, IndividualAddress, KnxError, Result};

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum CemiMessageCode {
    LDataRequest = 0x11,
    LDataIndication = 0x29,
    LDataConfirmation = 0x2e,
}

// All `CemiMessageCode` variants in declaration order; together with `as_u8`
// this is the single source of truth for the variant <-> byte mapping.
const CEMI_MESSAGE_CODE_ALL: [CemiMessageCode; 3] = [
    CemiMessageCode::LDataRequest,
    CemiMessageCode::LDataIndication,
    CemiMessageCode::LDataConfirmation,
];

impl CemiMessageCode {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for CemiMessageCode {
    type Error = KnxError;

    fn try_from(value: u8) -> Result<Self> {
        CEMI_MESSAGE_CODE_ALL
            .iter()
            .copied()
            .find(|mc| mc.as_u8() == value)
            .ok_or(KnxError::InvalidFrame("unsupported cEMI message code"))
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupTelegram {
    source: IndividualAddress,
    destination: GroupAddress,
    apci: Apci,
    payload: std::vec::Vec<u8>,
}

impl GroupTelegram {
    /// Builds a validated group telegram.
    ///
    /// Payload rules:
    /// - `GroupValueRead` carries no payload (a non-empty payload is rejected).
    /// - `GroupValueWrite` / `GroupValueResponse` must carry a payload. An
    ///   empty Write/Response payload is rejected because it is ambiguous with
    ///   the compact one-byte zero value on APDU decode (a 2-byte compact
    ///   APDU always decodes back to `[0x00]`, never to an empty payload), so
    ///   an empty Write/Response could not round-trip distinctly. `[0x00]`
    ///   remains a valid compact one-byte value payload.
    /// - Payloads longer than 254 bytes are rejected.
    pub fn new(
        source: IndividualAddress,
        destination: GroupAddress,
        apci: Apci,
        payload: &[u8],
    ) -> Result<Self> {
        if matches!(apci, Apci::GroupValueRead) && !payload.is_empty() {
            return Err(KnxError::InvalidFrame("group read cannot carry payload"));
        }
        if !matches!(apci, Apci::GroupValueRead) && payload.is_empty() {
            return Err(KnxError::InvalidFrame(
                "group write/response must carry payload",
            ));
        }
        if payload.len() > 254 {
            return Err(KnxError::InvalidFrame("group payload too long"));
        }

        Ok(Self {
            source,
            destination,
            apci,
            payload: payload.to_vec(),
        })
    }

    pub const fn source(&self) -> IndividualAddress {
        self.source
    }

    pub const fn destination(&self) -> GroupAddress {
        self.destination
    }

    pub const fn apci(&self) -> Apci {
        self.apci
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    fn encode_apdu(&self, out: &mut std::vec::Vec<u8>) -> Result<()> {
        out.push(0x00);

        match self.payload.as_slice() {
            [] => out.push(self.apci.service_bits()),
            [short] if *short <= 0x3f => out.push(self.apci.service_bits() | short),
            payload => {
                out.push(self.apci.service_bits());
                out.extend_from_slice(payload);
            }
        }

        Ok(())
    }

    fn decode_apdu(
        source: IndividualAddress,
        destination: GroupAddress,
        apdu: &[u8],
    ) -> Result<Self> {
        if apdu.len() < 2 {
            return Err(KnxError::BufferTooShort {
                needed: 2,
                actual: apdu.len(),
            });
        }
        if apdu[0] != 0x00 {
            return Err(KnxError::InvalidFrame("unsupported TPCI field"));
        }

        let apci = Apci::try_from(apdu[1])?;
        let payload = if apdu.len() == 2 {
            match apci {
                Apci::GroupValueRead => std::vec::Vec::new(),
                Apci::GroupValueResponse | Apci::GroupValueWrite => {
                    std::vec::Vec::from([apdu[1] & 0x3f])
                }
            }
        } else {
            if apdu[1] & 0x3f != 0 {
                return Err(KnxError::InvalidFrame("extended APDU has inline data bits"));
            }
            apdu[2..].to_vec()
        };

        Self::new(source, destination, apci, &payload)
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CemiFrame {
    message_code: CemiMessageCode,
    control1: u8,
    control2: u8,
    telegram: GroupTelegram,
    /// Opaque cEMI additional-info block, preserved verbatim across
    /// decode/encode. Empty for frames built via the public constructors.
    /// Individual additional-info elements are intentionally not parsed yet.
    additional_info: std::vec::Vec<u8>,
}

impl CemiFrame {
    pub const DEFAULT_CONTROL1: u8 = 0xbc;
    pub const DEFAULT_CONTROL2_GROUP: u8 = 0xe0;

    pub const fn new(message_code: CemiMessageCode, telegram: GroupTelegram) -> Self {
        Self {
            message_code,
            control1: Self::DEFAULT_CONTROL1,
            control2: Self::DEFAULT_CONTROL2_GROUP,
            telegram,
            additional_info: std::vec::Vec::new(),
        }
    }

    pub fn group_value_read(source: IndividualAddress, destination: GroupAddress) -> Result<Self> {
        Ok(Self::new(
            CemiMessageCode::LDataRequest,
            GroupTelegram::new(source, destination, Apci::GroupValueRead, &[])?,
        ))
    }

    pub fn group_value_response(
        source: IndividualAddress,
        destination: GroupAddress,
        payload: &[u8],
    ) -> Result<Self> {
        Ok(Self::new(
            CemiMessageCode::LDataRequest,
            GroupTelegram::new(source, destination, Apci::GroupValueResponse, payload)?,
        ))
    }

    pub fn group_value_write(
        source: IndividualAddress,
        destination: GroupAddress,
        payload: &[u8],
    ) -> Result<Self> {
        Ok(Self::new(
            CemiMessageCode::LDataRequest,
            GroupTelegram::new(source, destination, Apci::GroupValueWrite, payload)?,
        ))
    }

    pub const fn message_code(&self) -> CemiMessageCode {
        self.message_code
    }

    pub const fn control1(&self) -> u8 {
        self.control1
    }

    pub const fn control2(&self) -> u8 {
        self.control2
    }

    pub const fn telegram(&self) -> &GroupTelegram {
        &self.telegram
    }

    /// Opaque cEMI additional-info bytes (empty for constructor-built frames).
    pub fn additional_info(&self) -> &[u8] {
        &self.additional_info
    }

    /// Returns the frame with the given opaque additional-info block.
    ///
    /// The cEMI additional-info length is a single octet, so the block must
    /// be at most 255 bytes; longer input is rejected with
    /// `KnxError::InvalidFrame("additional info too long")`.
    pub fn with_additional_info(
        mut self,
        additional_info: impl Into<std::vec::Vec<u8>>,
    ) -> Result<Self> {
        let additional_info = additional_info.into();
        if u8::try_from(additional_info.len()).is_err() {
            return Err(KnxError::InvalidFrame("additional info too long"));
        }
        self.additional_info = additional_info;
        Ok(self)
    }

    pub fn decode(input: &[u8]) -> Result<(Self, &[u8])> {
        if input.len() < 2 {
            return Err(KnxError::BufferTooShort {
                needed: 2,
                actual: input.len(),
            });
        }

        let message_code = CemiMessageCode::try_from(input[0])?;
        let additional_info_len = input[1] as usize;
        let fixed_start = 2 + additional_info_len;
        let fixed_len = fixed_start + 7;

        if input.len() < fixed_len {
            return Err(KnxError::BufferTooShort {
                needed: fixed_len,
                actual: input.len(),
            });
        }

        // input.len() >= fixed_len >= fixed_start, so this slice is in-bounds.
        let additional_info = input[2..fixed_start].to_vec();
        let control1 = input[fixed_start];
        let control2 = input[fixed_start + 1];
        let source = IndividualAddress::from_raw(u16::from_be_bytes([
            input[fixed_start + 2],
            input[fixed_start + 3],
        ]));
        let destination = GroupAddress::from_raw(u16::from_be_bytes([
            input[fixed_start + 4],
            input[fixed_start + 5],
        ]));
        let apdu_len = input[fixed_start + 6] as usize + 1;
        let apdu_start = fixed_start + 7;
        let needed = apdu_start + apdu_len;

        if input.len() < needed {
            return Err(KnxError::BufferTooShort {
                needed,
                actual: input.len(),
            });
        }

        let telegram = GroupTelegram::decode_apdu(source, destination, &input[apdu_start..needed])?;

        Ok((
            Self {
                message_code,
                control1,
                control2,
                telegram,
                additional_info,
            },
            &input[needed..],
        ))
    }

    pub fn encode(&self, out: &mut std::vec::Vec<u8>) -> Result<()> {
        let mut apdu = std::vec::Vec::new();
        self.telegram.encode_apdu(&mut apdu)?;
        let npdu_len = apdu
            .len()
            .checked_sub(1)
            .and_then(|value| u8::try_from(value).ok())
            .ok_or(KnxError::InvalidFrame("APDU length out of range"))?;
        let additional_info_len = u8::try_from(self.additional_info.len())
            .map_err(|_| KnxError::InvalidFrame("additional info too long"))?;

        out.push(self.message_code.as_u8());
        out.push(additional_info_len);
        out.extend_from_slice(&self.additional_info);
        out.extend_from_slice(&[
            self.control1,
            self.control2,
            (self.telegram.source().raw() >> 8) as u8,
            self.telegram.source().raw() as u8,
            (self.telegram.destination().raw() >> 8) as u8,
            self.telegram.destination().raw() as u8,
            npdu_len,
        ]);
        out.extend_from_slice(&apdu);

        Ok(())
    }
}
