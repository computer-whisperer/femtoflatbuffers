use crate::{Decoder, Encoder, EncodeError, DecodeError};

pub trait Table {
    fn encode(&self, encoder: &mut Encoder) -> Result<(), EncodeError>;
    fn decode(decoder: &Decoder) -> Result<Self, DecodeError> where Self: Sized;
}