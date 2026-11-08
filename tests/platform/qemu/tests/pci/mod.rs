#[path = "../../pci/access.rs"]
mod access;
use access::Config;
#[allow(dead_code)]
#[path = "../../pci/endpoint.rs"]
pub(crate) mod endpoint;
mod pci {
    pub(crate) use crate::endpoint;
    pub(crate) use crate::{Config, endpoint::Endpoint};
}
mod cases;
