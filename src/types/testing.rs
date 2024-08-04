use super::machine::Output;
use crate::{FinishStatus, Metadata};
use std::error::Error;

pub trait ResultUtils {
	fn is_accepted(&self) -> bool;
	fn is_rejected(&self) -> bool;
	fn is_errored(&self) -> bool;
	fn get_error(&self) -> Option<&dyn Error>;
	fn get_outputs(&self) -> Vec<Output>;
}

#[derive(Debug)]
pub struct AdvanceResult {
	pub outputs: Vec<Output>,
	pub metadata: Metadata,
	pub status: FinishStatus,
	pub error: Option<Box<dyn Error>>,
}

impl AdvanceResult {
	pub fn get_metadata(&self) -> &Metadata {
		&self.metadata
	}
}

#[derive(Debug)]
pub struct InspectResult {
	pub outputs: Vec<Output>,
	pub status: FinishStatus,
	pub error: Option<Box<dyn Error>>,
}

impl ResultUtils for AdvanceResult {
	fn is_accepted(&self) -> bool {
		self.status == FinishStatus::Accept
	}

	fn is_rejected(&self) -> bool {
		self.status == FinishStatus::Reject
	}

	fn is_errored(&self) -> bool {
		self.error.is_some()
	}

	fn get_error(&self) -> Option<&dyn Error> {
		self.error.as_deref()
	}

	fn get_outputs(&self) -> Vec<Output> {
		self.outputs.clone()
	}
}

impl ResultUtils for InspectResult {
	fn is_accepted(&self) -> bool {
		self.status == FinishStatus::Accept
	}

	fn is_rejected(&self) -> bool {
		self.status == FinishStatus::Reject
	}

	fn is_errored(&self) -> bool {
		self.error.is_some()
	}

	fn get_error(&self) -> Option<&dyn Error> {
		self.error.as_deref()
	}

	fn get_outputs(&self) -> Vec<Output> {
		self.outputs.clone()
	}
}
