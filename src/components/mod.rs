use std::error::Error;

use async_trait::async_trait;

use crate::types::BackupDescriptor;

pub mod common;
pub mod operate;
pub mod zeebe;

type BackupId = u64;
pub trait EndpointError: Error + Send {}

#[async_trait]
pub trait Endpoint {
    type Error: EndpointError;
    async fn request(
        &self,
        req: hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<hyper::Body>, Self::Error>;
}

pub enum BackupError<E: EndpointError> {
    AlreadyExists,
    NotFound,
    Failed,
    Endpoint(E),
    UnexpectedJson(serde_json::Error),
    HttpError(hyper::Error),
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
