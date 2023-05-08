use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::types::{BackupDescriptor, TakeBackupRequest};

use super::{Backup, BackupError, BackupId, Endpoint};

pub trait Component: Sync + Send {
    type Endpoint: Endpoint;
    type Details;

    fn endpoint(&self) -> &Self::Endpoint;
}

#[async_trait]
impl<C: ?Sized> Backup for C
where
    C: Component + Sync,
    C::Endpoint: Send + Sync,
    C::Details: DeserializeOwned,
{
    type Endpoint = <Self as Component>::Endpoint;
    type Details = <Self as Component>::Details;

    async fn query(
        &self,
        id: BackupId,
    ) -> Result<BackupDescriptor<Self::Details>, BackupError<<Self::Endpoint as Endpoint>::Error>>
    {
        let req: hyper::Request<hyper::Body> = hyper::Request::builder()
            .method(hyper::Method::GET)
            .uri(format!("/actuator/backups/{}", id))
            .body(hyper::Body::empty())
            .expect("Reqest must be valid");

        match self.endpoint().request(req).await {
            Ok(mut resp) => {
                let body = hyper::body::to_bytes(resp.body_mut()).await?;
                let details = serde_json::from_slice(&body)?;
                Ok(details)
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn list(
        &self,
    ) -> Result<
        Vec<BackupDescriptor<Self::Details>>,
        BackupError<<Self::Endpoint as Endpoint>::Error>,
    > {
        let req: hyper::Request<hyper::Body> = hyper::Request::builder()
            .method(hyper::Method::GET)
            .uri("/actuator/backups")
            .body(hyper::Body::empty())
            .expect("Reqest must be valid");

        match self.endpoint().request(req).await {
            Ok(mut resp) => {
                let body = hyper::body::to_bytes(resp.body_mut()).await?;
                let details = serde_json::from_slice(&body)?;
                Ok(details)
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn create(
        &self,
        id: BackupId,
    ) -> Result<BackupDescriptor<Self::Details>, BackupError<<Self::Endpoint as Endpoint>::Error>>
    {
        let req: hyper::Request<hyper::Body> = hyper::Request::builder()
            .method(hyper::Method::POST)
            .uri("/actuator/backups")
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .body(
                serde_json::to_string(&TakeBackupRequest {
                    backup_id: id.to_string(),
                })
                .expect("Request can be serialized")
                .into(),
            )
            .expect("Reqest must be valid");

        match self.endpoint().request(req).await {
            Ok(mut resp) => {
                let body = hyper::body::to_bytes(resp.body_mut()).await?;
                let details = serde_json::from_slice(&body)?;
                Ok(details)
            }
            Err(e) => Err(e.into()),
        }
    }
    async fn delete(
        &self,
        id: BackupId,
    ) -> Result<(), BackupError<<Self::Endpoint as Endpoint>::Error>> {
        let req: hyper::Request<hyper::Body> = hyper::Request::builder()
            .method(hyper::Method::DELETE)
            .uri(format!("/actuator/backups/{}", id))
            .header(hyper::header::CONTENT_TYPE, "application/json")
            .body(hyper::Body::empty())
            .expect("Reqest must be valid");

        match self.endpoint().request(req).await {
            Ok(mut resp) => {
                let body = hyper::body::to_bytes(resp.body_mut()).await?;
                let details = serde_json::from_slice(&body)?;
                Ok(details)
            }
            Err(e) => Err(e.into()),
        }
    }
}
