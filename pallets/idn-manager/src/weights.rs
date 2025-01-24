use frame_support::weights::Weight;

pub trait WeightInfo {
	fn create_subscription() -> Weight;
}
