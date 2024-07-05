use std::path::PathBuf;
use thiserror::Error;

pub struct ErrorCodes;

impl ErrorCodes {
    pub const PROCESS_EXIT: i32 = 42;

    pub const CONFIG_MISSING: u32 = 1001;
    pub const SERIALIZATION_ERROR: u32 = 1002;
    pub const IO_ERROR: u32 = 1003;
    pub const ILLEGAL_NODE_ID: u32 = 1004;
}

pub type OperatorConfigResult<T> = Result<T, OperatorConfigError>;

#[derive(Error, Debug)]
pub enum OperatorConfigError {
    #[error(
        "No operator config found at this path: {0} (Error Code: {})",
        ErrorCodes::CONFIG_MISSING
    )]
    ConfigMissing(PathBuf),

    #[error(
        "Config deserialization error: {0} (Error Code: {})",
        ErrorCodes::SERIALIZATION_ERROR
    )]
    SerializationError(#[from] serde_yaml::Error),

    #[error(
        "Error while performing IO for the Operator: {0} (Error Code: {})",
        ErrorCodes::IO_ERROR
    )]
    IoError(#[from] std::io::Error),

    #[error(
        "Error nodeid illegal, must be hex format, and 64 bits (Error Code: {})",
        ErrorCodes::ILLEGAL_NODE_ID
    )]
    IllegalNodeId,
}

pub type OperatorResult<T> = Result<T, OperatorError>;

#[derive(Error, Debug)]
pub enum OperatorError {}
