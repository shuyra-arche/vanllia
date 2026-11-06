use crate::reason::Reason;
use std::collections::BTreeMap;

/// An invocation's in-memory choice. It is never serialized as configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deployment {
    pub system: String,
    pub order: Vec<String>,
    pub reasons: BTreeMap<String, Vec<Reason>>,
}
