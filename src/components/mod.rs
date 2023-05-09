use std::{error::Error, fmt::Display};

use async_trait::async_trait;

use crate::types::BackupDescriptor;

pub mod common;
pub mod operate;
pub mod zeebe;

type BackupId = u64;
pub trait EndpointError: Error + Send + 'static {}

#[async_trait]
pub trait Endpoint: Send + Sync {
    type Error: EndpointError;
    async fn request(
        &self,
        req: hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<hyper::Body>, Self::Error>;
}

#[derive(Debug)]
pub enum BackupError<E: EndpointError> {
    AlreadyExists,
    NotFound,
    Failed,
    Endpoint(E),
    UnexpectedJson(serde_json::Error),
    HttpError(hyper::Error),
}

impl<E> Display for BackupError<E>
where
    E: EndpointError,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupError::AlreadyExists => write!(f, "Backup already exists"),
            BackupError::NotFound => write!(f, "Backup not found"),
            BackupError::Failed => write!(f, "Backup failed"),
            BackupError::Endpoint(e) => write!(f, "Endpoint error: {}", e),
            BackupError::UnexpectedJson(e) => write!(f, "Unexpected JSON: {}", e),
            BackupError::HttpError(e) => write!(f, "HTTP error: {}", e),
        }
    }
}

impl<E> Error for BackupError<E>
where
    E: EndpointError,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            BackupError::AlreadyExists => None,
            BackupError::NotFound => None,
            BackupError::Failed => None,
            BackupError::Endpoint(e) => Some(e),
            BackupError::UnexpectedJson(e) => Some(e),
            BackupError::HttpError(e) => Some(e),
        }
    }
}

impl<E> From<serde_json::Error> for BackupError<E>
where
    E: EndpointError,
{
    fn from(e: serde_json::Error) -> Self {
        Self::UnexpectedJson(e)
    }
}

impl<E> From<hyper::Error> for BackupError<E>
where
    E: EndpointError,
{
    fn from(e: hyper::Error) -> Self {
        Self::HttpError(e)
    }
}

impl<E> From<E> for BackupError<E>
where
    E: EndpointError,
{
    fn from(e: E) -> Self {
        Self::Endpoint(e)
    }
}

#[async_trait]
pub trait Backup {
    type Endpoint: Endpoint;
    type Details;

    async fn query(
        &self,
        id: BackupId,
    ) -> Result<BackupDescriptor<Self::Details>, BackupError<<Self::Endpoint as Endpoint>::Error>>;
    async fn list(
        &self,
    ) -> Result<
        Vec<BackupDescriptor<Self::Details>>,
        BackupError<<Self::Endpoint as Endpoint>::Error>,
    >;
    async fn create(
        &self,
        id: BackupId,
    ) -> Result<BackupDescriptor<Self::Details>, BackupError<<Self::Endpoint as Endpoint>::Error>>;
    async fn delete(
        &self,
        id: BackupId,
    ) -> Result<(), BackupError<<Self::Endpoint as Endpoint>::Error>>;
}

enum RestoreError {
    NotFound,
    Failed,
    HttpError(hyper::Error),
}

#[async_trait]
trait Restore {
    type Endpoint: Endpoint;
    async fn restore(&self, id: BackupId) -> Result<(), RestoreError>;
}
