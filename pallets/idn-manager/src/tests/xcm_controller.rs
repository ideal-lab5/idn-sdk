/*
 * Copyright 2025 by Ideal Labs, LLC
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! Mock ExecuteController for fine-grained conditional error injection for xcm testing

use frame_support::{dispatch::PostDispatchInfo, parameter_types};
use xcm::{latest::prelude::*, VersionedLocation, VersionedXcm, VersionedXcm::V5};
#[cfg(feature = "runtime-benchmarks")]
use xcm_builder::test_utils::Response;
use xcm_builder::{
	test_utils::{QueryId, SendXcm, Xcm as TestXcm},
	ExecuteController, QueryController, QueryHandler, SendController,
};
use xcm_executor::traits::QueryResponseStatus;

use sp_runtime::{DispatchError, DispatchErrorWithPostInfo};
use sp_weights::Weight;

pub struct TestController;

// ExecuteController
impl<Origin, RuntimeCall> ExecuteController<Origin, RuntimeCall> for TestController
where
	Origin: Clone,
{
	type WeightInfo = ();

	fn execute(
		_origin: Origin,
		_message: Box<VersionedXcm<RuntimeCall>>,
		_max_weight: Weight,
	) -> Result<Weight, DispatchErrorWithPostInfo<PostDispatchInfo>> {
		// Always succeed
		Ok(Weight::from_parts(1_000_000_000, 0))
	}
}

// Validation always fails when parent = 42
impl SendXcm for TestController {
	type Ticket = ();
	fn validate(
		destination: &mut Option<Location>,
		_message: &mut Option<TestXcm<()>>,
	) -> SendResult<()> {
		match destination.as_ref().ok_or(SendError::MissingArgument)?.unpack() {
			(42, []) => Err(SendError::NotApplicable),
			_ => Ok(((), Assets::new())),
		}
	}
	fn deliver(_: ()) -> Result<XcmHash, SendError> {
		Ok([0; 32])
	}
}

// SendController
impl<Origin> SendController<Origin> for TestController
where
	Origin: Clone,
{
	type WeightInfo = ();
	fn send(
		_origin: Origin,
		destination: Box<VersionedLocation>,
		message: Box<VersionedXcm<()>>,
	) -> Result<XcmHash, DispatchError> {
		let mut loc: Option<TestXcm<()>> = None;
		if let V5(xcm) = *message { loc = Some(xcm) }
		let (ticket, _assets) = <Self as SendXcm>::validate(
			&mut Some(destination.as_ref().try_as::<Location>().unwrap().clone()),
			&mut loc,
		)
		.map_err(|_| DispatchError::Other("SendXcm Validation Failed"))?;
		<Self as SendXcm>::deliver(ticket)
			.map_err(|_| DispatchError::Other("SendXcm Deliver Failed"))
	}
}

parameter_types! {
	pub static UniversalLocation: InteriorLocation
		= (ByGenesis([0; 32]), Parachain(42)).into();
}

impl QueryHandler for TestController {
	type BlockNumber = u64;
	type Error = ();
	type UniversalLocation = UniversalLocation;

	fn new_query(
		_responder: impl Into<Location>,
		_timeout: Self::BlockNumber,
		_match_querier: impl Into<Location>,
	) -> u64 {
		0
	}

	fn report_outcome(
		_message: &mut TestXcm<()>,
		_responder: impl Into<Location>,
		_timeout: Self::BlockNumber,
	) -> Result<u64, Self::Error> {
		Ok(0)
	}

	fn take_response(_id: u64) -> QueryResponseStatus<Self::BlockNumber> {
		QueryResponseStatus::NotFound
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn expect_response(_id: QueryId, _response: Response) {}
}

impl<Origin, Timeout> QueryController<Origin, Timeout> for TestController
where
	Origin: Clone,
{
	type WeightInfo = ();
	fn query(
		_origin: Origin,
		_timeout: Timeout,
		_match_querier: VersionedLocation,
	) -> Result<QueryId, DispatchError> {
		Ok(0u64)
	}
}
