use crate::types::OperateDetails;

use super::{common::Component, Endpoint};

#[derive(Debug)]
pub struct Operate<E: Endpoint> {
    pub(crate) endpoint: E,
}

impl<E> Component for Operate<E>
where
    E: Endpoint + Send + Sync,
{
    type Endpoint = E;

    type Details = OperateDetails;

    fn endpoint(&self) -> &Self::Endpoint {
        &self.endpoint
    }
}
