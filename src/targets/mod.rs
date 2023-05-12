use std::{
    error::Error,
    fmt::{Debug, Display},
};

use async_trait::async_trait;

use crate::{
    components::Component,
    types::{OperateDetails, ZeebeDetails},
};

pub mod remote;

pub trait Target<E>: Display + Debug
where
    E: Endpoint,
{
    fn zeebe(&self) -> &dyn Component<Endpoint = E, Details = ZeebeDetails>;
    fn operate(&self) -> &dyn Component<Endpoint = E, Details = OperateDetails>;
}

pub trait EndpointError: Error + Send + 'static {}

#[async_trait]
pub trait Endpoint: Send + Sync {
    type Error: EndpointError;
    async fn request(
        &self,
        req: hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<hyper::Body>, Self::Error>;
}
