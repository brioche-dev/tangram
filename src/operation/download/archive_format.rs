use std::path::Path;

pub enum ArchiveFormat {
	Tar(Option<CompressionFormat>),
	Zip,
}

pub enum CompressionFormat {
	Bz2,
	Gz,
	Lz,
	Xz,
	Zstd,
}

impl ArchiveFormat {
	#[allow(clippy::case_sensitive_file_extension_comparisons)]
	pub fn for_path(path: &Path) -> Option<ArchiveFormat> {
		let path = path.to_str().unwrap();
		if path.ends_with(".tar.bz2") || path.ends_with(".tbz2") {
			Some(ArchiveFormat::Tar(Some(CompressionFormat::Bz2)))
		} else if path.ends_with(".tar.gz") || path.ends_with(".tgz") {
			Some(ArchiveFormat::Tar(Some(CompressionFormat::Gz)))
		} else if path.ends_with(".tar.lz") || path.ends_with(".tlz") {
			Some(ArchiveFormat::Tar(Some(CompressionFormat::Lz)))
		} else if path.ends_with(".tar.xz") || path.ends_with(".txz") {
			Some(ArchiveFormat::Tar(Some(CompressionFormat::Xz)))
		} else if path.ends_with(".tar.zstd")
			|| path.ends_with(".tzstd")
			|| path.ends_with(".tar.zst")
			|| path.ends_with(".tzst")
		{
			Some(ArchiveFormat::Tar(Some(CompressionFormat::Zstd)))
		} else if path.ends_with(".tar") {
			Some(ArchiveFormat::Tar(None))
		} else if path.ends_with(".zip") {
			Some(ArchiveFormat::Zip)
		} else {
			None
		}
	}
}
