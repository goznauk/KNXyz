use knx_core::{Apci, CemiFrame, GroupAddress, IndividualAddress};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupEvent {
    pub source: IndividualAddress,
    pub destination: GroupAddress,
    pub apci: Apci,
    pub payload: Vec<u8>,
}

impl GroupEvent {
    pub fn from_cemi(frame: &CemiFrame) -> Self {
        let telegram = frame.telegram();

        Self {
            source: telegram.source(),
            destination: telegram.destination(),
            apci: telegram.apci(),
            payload: telegram.payload().to_vec(),
        }
    }
}
