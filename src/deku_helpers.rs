use deku::{ctx::Endian, no_std_io, prelude::*};
use jiff::Timestamp;

pub fn reader_realtime<R: no_std_io::Read + no_std_io::Seek>(
	reader: &mut Reader<R>,
) -> Result<Timestamp, DekuError> {
	let value = u64::from_reader_with_ctx(reader, Endian::Little)?;
	Timestamp::from_microsecond(value.try_into()?)
		.map_err(|err| DekuError::Assertion(format!("Invalid timestamp: {err}").into()))
}

pub fn writer_realtime<W: std::io::Write + std::io::Seek>(
	writer: &mut Writer<W>,
	field: &Timestamp,
) -> Result<(), DekuError> {
	let value: u64 = field.as_microsecond().try_into()?;
	value.to_writer(writer, Endian::Little)
}

pub fn reader_realtime_opt<R: no_std_io::Read + no_std_io::Seek>(
	reader: &mut Reader<R>,
) -> Result<Option<Timestamp>, DekuError> {
	let value = u64::from_reader_with_ctx(reader, Endian::Little)?;
	Timestamp::from_microsecond(value.try_into()?)
		.map_err(|err| DekuError::Assertion(format!("Invalid timestamp: {err}").into()))
		.map(Some)
}

pub fn writer_realtime_opt<W: std::io::Write + std::io::Seek>(
	writer: &mut Writer<W>,
	field: &Option<Timestamp>,
) -> Result<(), DekuError> {
	let value: u64 = field
		.map(|ts| ts.as_microsecond())
		.unwrap_or_default()
		.try_into()?;
	value.to_writer(writer, Endian::Little)
}
