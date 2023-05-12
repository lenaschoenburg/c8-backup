use crate::types::ZeebeDetails;

use super::{Component, Endpoint};

#[derive(Debug)]
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
