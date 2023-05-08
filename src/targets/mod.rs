use std::error::Error;

use async_trait::async_trait;
use k8s_openapi::api::core::v1::Pod;

use crate::{
    components::{common::Component, Endpoint, EndpointError},
    types::{OperateDetails, ZeebeDetails},
};

pub mod remote;

#[async_trait]
pub trait Target<E>
where
    E: Endpoint,
{
    async fn zeebe(&self) -> &dyn Component<Endpoint = E, Details = ZeebeDetails>;
    async fn operate(&self) -> &dyn Component<Endpoint = E, Details = OperateDetails>;
}
