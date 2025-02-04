use frame_support::weights::Weight;

pub trait WeightInfo {
	fn create_subscription() -> Weight;
}

impl WeightInfo for () {
	fn create_subscription() -> Weight {
		Weight::from_parts(2_956_000, 1627)
	}
}
