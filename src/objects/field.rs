use deku::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct FieldObjectHeader {
	pub hash: u64,
	pub next_hash_offset: u64,
	pub next_data_offset: u64,
}
