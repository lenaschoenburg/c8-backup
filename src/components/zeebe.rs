use std::{error::Error, fmt::Display};

use async_trait::async_trait;
use hyper::{Body, Method, Request, Response};
use k8s_openapi::api::core::v1::Pod;
use kube::{api::ListParams, Api};
use tracing::{debug, error};

use crate::types::ZeebeDetails;

use super::{
    common::Component, Backup, BackupError, BackupId, Endpoint, EndpointError, Restore,
    RestoreError,
};

pub struct Zeebe<E: Endpoint> {
    pub(crate) endpoint: E,
}

impl<E> Component for Zeebe<E>
where
    E: Endpoint + Send + Sync,
{
    type Endpoint = E;

    type Details = ZeebeDetails;

    fn endpoint(&self) -> &Self::Endpoint {
        &self.endpoint
    }
}
