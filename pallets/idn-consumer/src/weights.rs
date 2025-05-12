use frame_support::weights::Weight;
pub trait WeightInfo {
	fn consume_pulse() -> Weight;
	fn consume_quote() -> Weight;
	fn consume_sub_info() -> Weight;
}

impl WeightInfo for () {
	fn consume_pulse() -> Weight {
		Weight::from_parts(10_000, 1)
	}
	fn consume_quote() -> Weight {
		Weight::from_parts(10_000, 1)
	}
	fn consume_sub_info() -> Weight {
		Weight::from_parts(10_000, 1)
	}
}
