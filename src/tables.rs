use std::num::NonZeroU64;

use deku::prelude::*;

// used for both data and field hash tables
// the hash table is an array of these
// key is hash % size of the hash table
#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "little")]
pub struct HashItem {
	pub head_hash_offset: Option<NonZeroU64>,
	pub tail_hash_offset: Option<NonZeroU64>,
}
