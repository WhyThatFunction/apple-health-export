use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("address parse error: {0}")]
    Addr(#[from] std::net::AddrParseError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("utf8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("byte stream error: {0}")]
    ByteStream(#[from] aws_smithy_types::byte_stream::error::Error),

    #[error("s3 get error: {0}")]
    S3Get(
        #[from] Box<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::get_object::GetObjectError>>,
    ),

    #[error("s3 put error: {0}")]
    S3Put(
        #[from] Box<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::put_object::PutObjectError>>,
    ),

    #[error("s3 head bucket error: {0}")]
    S3Head(
        #[from]
        Box<aws_sdk_s3::error::SdkError<aws_sdk_s3::operation::head_bucket::HeadBucketError>>,
    ),

    #[error("ExporterBuildError error: {source}")]
    ExporterBuild {
        #[from]
        source: opentelemetry_otlp::ExporterBuildError,
    },

    #[error("SetGlobalDefaultError error: {source}")]
    SetGlobalDefault {
        #[from]
        source: tracing::dispatcher::SetGlobalDefaultError,
    },
}
