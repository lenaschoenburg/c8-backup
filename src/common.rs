use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper_util::rt::TokioIo;
use k8s_openapi::api::core::v1::Pod;
use kube::{api::ListParams, Api};
use tracing::{debug, error};

#[tracing::instrument(skip(kube), err, level = "debug")]
pub async fn make_component_request(
    kube: &kube::Client,
    component: &str,
    port: u16,
    mut req: Request<Full<Bytes>>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    let pods = Api::<Pod>::default_namespaced(kube.clone());
    let pod = pods
        .list(&ListParams::default().labels(component))
        .await?
        .items
        .first()
        .expect(&format!("Pod with label {component} must exist"))
        .clone();
    let forwarded_port = pods
        .portforward(&pod.metadata.name.expect("Pod must have a name"), &[port])
        .await?
        .take_stream(port)
        .expect(&format!("Port {port} must be open"));

    let io = TokioIo::new(forwarded_port);
    let (mut sender, connection) = hyper::client::conn::http1::handshake(io).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("Failed to establish connection: {}", e);
        } else {
            debug!("Connection established");
        }
    });

    req.headers_mut()
        .append("Host", "127.0.0.1".parse().unwrap());

    let mut resp = sender.send_request(req).await?;
    if !resp.status().is_success() {
        let body = resp.body_mut().collect().await.map(|c| c.to_bytes());
        error!("Request failed: {:?}, {:?}", resp, body);
        return Err("Request failed".into());
    }
    let body = resp.body_mut().collect().await?.to_bytes();
    Ok(body)
}
