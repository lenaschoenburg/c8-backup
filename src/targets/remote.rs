use std::{error::Error, fmt::Display};

use async_trait::async_trait;
use k8s_openapi::api::core::v1::Pod;
use kube::{api::ListParams, Api};
use tracing::{debug, error};

use crate::{
    components::{
        self, common::Component, operate::Operate, zeebe::Zeebe, Endpoint, EndpointError,
    },
    types::{OperateDetails, ZeebeDetails},
};

use super::Target;

const OPERATE_LABEL: &str = "app.kubernetes.io/component=operate";
const ZEEBE_LABEL: &str = "app.kubernetes.io/component=operate";

pub struct RemoteHelmInstallation {
    kube: kube::Client,
    zeebe: Zeebe<RemotePod>,
    operate: Operate<RemotePod>,
}

impl RemoteHelmInstallation {
    pub async fn find_in_namespace(namespace: &str) -> Result<RemoteHelmInstallation, kube::Error> {
        let kube = kube::Client::try_default().await?;
        let pods = Api::<Pod>::namespaced(kube.clone(), namespace);

        let operate_name = pods
            .list(&ListParams::default().labels(OPERATE_LABEL))
            .await?
            .items
            .first()
            .expect(&format!("Pod with label {OPERATE_LABEL} must exist"))
            .clone()
            .metadata
            .name
            .expect("Pod must have a name");

        let zeebe_name = pods
            .list(&ListParams::default().labels(ZEEBE_LABEL))
            .await?
            .items
            .first()
            .expect(&format!("Pod with label {ZEEBE_LABEL} must exist"))
            .clone()
            .metadata
            .name
            .expect("Pod must have a name");

        Ok(RemoteHelmInstallation {
            kube: kube.clone(),
            zeebe: Zeebe {
                endpoint: RemotePod {
                    pods: pods.clone(),
                    port: 9600,
                    name: zeebe_name,
                },
            },
            operate: Operate {
                endpoint: RemotePod {
                    pods: pods.clone(),
                    port: 8080,
                    name: operate_name,
                },
            },
        })
    }
}

struct RemotePod {
    pods: Api<Pod>,
    name: String,
    port: u16,
}

#[derive(Debug)]
enum RemotePodEndpointError {
    KubeError(kube::Error),
    HyperError(hyper::Error),
}

impl EndpointError for RemotePodEndpointError {}

impl Error for RemotePodEndpointError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::KubeError(e) => Some(e),
            Self::HyperError(e) => Some(e),
        }
    }
}

impl Display for RemotePodEndpointError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<kube::Error> for RemotePodEndpointError {
    fn from(e: kube::Error) -> Self {
        Self::KubeError(e)
    }
}

impl From<hyper::Error> for RemotePodEndpointError {
    fn from(e: hyper::Error) -> Self {
        Self::HyperError(e)
    }
}

#[async_trait]
impl Endpoint for RemotePod {
    type Error = RemotePodEndpointError;

    async fn request(
        &self,
        mut req: hyper::Request<hyper::Body>,
    ) -> Result<hyper::Response<hyper::Body>, Self::Error> {
        let forwarded_port = self
            .pods
            .portforward(&self.name, &[self.port])
            .await?
            .take_stream(self.port)
            .expect(&format!("Port {} must be open", self.port));

        let (mut sender, connection) = hyper::client::conn::handshake(forwarded_port).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                error!("Failed to establish connection: {}", e);
            } else {
                debug!("Connection established");
            }
        });

        req.headers_mut()
            .append("Host", "127.0.0.1".parse().unwrap());

        Ok(sender.send_request(req).await?)
    }
}

#[async_trait]
impl Target<RemotePod> for RemoteHelmInstallation {
    async fn zeebe(&self) -> &dyn Component<Endpoint = RemotePod, Details = ZeebeDetails> {
        &self.zeebe
    }

    async fn operate(&self) -> &dyn Component<Endpoint = RemotePod, Details = OperateDetails> {
        &self.operate
    }
}
