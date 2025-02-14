use frame_support::weights::Weight;

pub trait WeightInfo {
	fn create_subscription() -> Weight;
	fn pause_subscription() -> Weight;
	fn reactivate_subscription() -> Weight;
	fn kill_subscription() -> Weight;
	fn update_subscription() -> Weight;
}

impl WeightInfo for () {
	fn create_subscription() -> Weight {
		Weight::from_parts(2_956_000, 1627)
	}
	fn pause_subscription() -> Weight {
		Weight::from_parts(2_956_000, 1627)
	}
	fn reactivate_subscription() -> Weight {
		Weight::from_parts(2_956_000, 1627)
	}
	fn kill_subscription() -> Weight {
		Weight::from_parts(2_956_000, 1627)
	}
	fn update_subscription() -> Weight {
		Weight::from_parts(2_956_000, 1627)
	}
}
