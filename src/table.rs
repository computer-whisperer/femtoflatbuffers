use crate::{DecodeError, Decoder, EncodeError, Encoder};

pub trait Table {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError>;
    fn decode(decoder: &Decoder) -> Result<Self, DecodeError>
    where
        Self: Sized;
}
