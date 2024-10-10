use std::num::NonZeroU64;

use deku::prelude::*;

use super::SimpleRead;

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayObjectHeader {
	#[deku(map = "|field: u64| -> Result<_, DekuError> { Ok(NonZeroU64::new(field)) }")]
	pub next_entry_array_offset: Option<NonZeroU64>,
}

pub const ENTRY_ARRAY_HEADER_SIZE: usize = std::mem::size_of::<EntryArrayObjectHeader>();
const _: [(); ENTRY_ARRAY_HEADER_SIZE] = [(); 8];

impl SimpleRead for EntryArrayObjectHeader {}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayRegularItem {
	pub offset: u64,
}

impl SimpleRead for EntryArrayRegularItem {}

#[derive(Debug, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct EntryArrayCompactItem {
	pub offset: u32,
}

impl SimpleRead for EntryArrayCompactItem {}
