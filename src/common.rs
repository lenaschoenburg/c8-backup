use hyper::{body::Bytes, Body, Request};
use k8s_openapi::api::core::v1::Pod;
use kube::{api::ListParams, Api};
use tracing::{debug, error};

pub async fn make_zeebe_request(
    kube: &kube::Client,
    req: Request<Body>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    make_component_request(kube, "app.kubernetes.io/component=zeebe-gateway", 9600, req).await
}

pub async fn make_operate_request(
    kube: &kube::Client,
    req: Request<Body>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    make_component_request(kube, "app.kubernetes.io/component=operate", 8080, req).await
}

pub async fn make_elasticsearch_request(
    kube: &kube::Client,
    req: Request<Body>,
) -> Result<Bytes, Box<dyn std::error::Error>> {
    make_component_request(kube, "app=elasticsearch-master", 9200, req).await
}

#[tracing::instrument(skip(kube), err, level = "debug")]
async fn make_component_request(
    kube: &kube::Client,
    component: &str,
    port: u16,
    mut req: Request<Body>,
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

    let mut resp = sender.send_request(req).await?;
    if !resp.status().is_success() {
        let body = hyper::body::to_bytes(resp.body_mut()).await;
        error!("Request failed: {:?}, {:?}", resp, body);
        return Err("Request failed".into());
    }
    let body = hyper::body::to_bytes(resp.body_mut()).await?;
    Ok(body)
}
