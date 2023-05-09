use std::fmt::{Debug, Display};

use crate::{
    components::{common::Component, Endpoint},
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
