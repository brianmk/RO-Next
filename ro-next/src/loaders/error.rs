use ragnarok_bytes::ConversionError;
use ro_next_loaders::FileNotFoundError;

#[derive(Debug)]
pub enum LoadError {
    File(FileNotFoundError),
    Conversion(Box<ConversionError>),
    UnsupportedFormat(String),
}
