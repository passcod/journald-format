use std::{num::NonZeroU64, time::Duration};

use deku::prelude::*;
use jiff::Timestamp;

/// Monotonic timestamp (microseconds).
///
/// On Linux, the epoch is the start of the system (boot). Corresponds to
/// [`CLOCK_MONOTONIC`](https://man7.org/linux/man-pages/man2/clock_gettime.2.html).
#[derive(Debug, Copy, Clone, PartialEq, Eq, DekuRead, DekuWrite)]
#[deku(endian = "endian", ctx = "endian: deku::ctx::Endian")]
pub struct Monotonic(pub NonZeroU64);

impl Monotonic {
	/// Create a monotonic if non-zero.
	///
	/// This mimics [`NonZeroU64::new`].
	pub fn new(ts: u64) -> Option<Self> {
		NonZeroU64::new(ts).map(Self)
	}

	/// Get as a timestamp given the epoch.
	pub fn to_timestamp(self, epoch: Timestamp) -> Timestamp {
		epoch.saturating_add(Duration::from_micros(self.0.get()))
	}
}
