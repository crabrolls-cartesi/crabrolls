use super::machine::Output;
use crate::{FinishStatus, Metadata};
use std::error::Error;

#[derive(Debug)]
pub struct AdvanceResult {
    pub outputs: Vec<Output>,
    pub metadata: Metadata,
    pub status: FinishStatus,
    pub error: Option<Box<dyn Error>>,
}

#[derive(Debug)]
pub struct InspectResult {
    pub outputs: Vec<Output>,
    pub status: FinishStatus,
    pub error: Option<Box<dyn Error>>,
}
