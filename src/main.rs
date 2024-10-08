use futures_util::StreamExt as _;
use journald_format::{
	impls::ReadWholeFile,
	reader::{JournalReader, Seek},
};
use tracing_subscriber::{
	fmt::format::FmtSpan, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
	// tracing_subscriber::registry()
	// 	.with(
	// 		EnvFilter::try_from_default_env()
	// 			.or_else(|_| EnvFilter::try_new("journald_format=trace"))
	// 			.unwrap(),
	// 	)
	// 	.with(tracing_subscriber::fmt::layer().with_span_events(FmtSpan::NEW | FmtSpan::CLOSE))
	// 	.init();

	let mut reader = JournalReader::new(ReadWholeFile::new("/var/log/journal".into()));

	let system = dbg!(reader.list().await?)
		.into_iter()
		.find(|s| s.scope == "system")
		.unwrap();

	reader.select(system).await?;
	reader.seek(Seek::Oldest).await?;

	let mut entries = reader.entries().take(1000);
	let mut total = 0;
	while let Some(entry) = entries.next().await {
		let entry = entry?;
		total += entry.objects.len();
	}

	dbg!(total);
	Ok(())
}
