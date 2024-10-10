use std::num::NonZeroU64;

use deku::prelude::*;

pub const TAG_LENGTH: u64 = 256 / 8;

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct TagObjectHeader {
	pub seqnum: NonZeroU64,
	pub epoch: u64,
	pub tag: [u8; TAG_LENGTH as _],
}
