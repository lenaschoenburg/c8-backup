use kube::Client;

/// Check if a Kubernetes cluster with Camunda is available.
/// Returns the client if available, None otherwise.
pub async fn try_kube_client() -> Option<Client> {
    Client::try_default().await.ok()
}
