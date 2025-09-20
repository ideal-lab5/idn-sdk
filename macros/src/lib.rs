use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};
use pallet_randomness_beacon::TemporalDirection;

// pub struct TimelockInput {

// }

#[proc_macro]
pub fn timelock(input: TokenStream) -> TokenStream {
	let args = parse_macro_input!(input as DeriveInput);

	// // TODO
	// if args.attrs.len() != 3 {
	// 	return syn::Error::new_spanned(
	// 		args.attrs,
	// 		"timelock_check requires exactly 3 arguments: round_expr, direction, error_variant",
	// 	)
	// 	.to_compile_error()
	// 	.into();
	// }

	let round_expr = &args.attrs[0];
	let direction = &args.attrs[1];
	let error_variant = &args.attrs[2];

	let expanded = quote! {
		if let Ok(direction) = self.env().extension().check_time(#round_expr) {
			if !#direction.eq(&direction) {
				return Err(#error_variant);
			}
		} else {
			return Err(Error::Void);
		}
	};

	TokenStream::from(expanded)
}

// #[cfg(test)]
// mod test {

//     pub struct Test {}

//     impl Test {
//         fn env() -> Self {
//             Self { }
//         }

//         fn extension() -> Self {
//             Self { }
//         }

//         fn check_time(when: u64) -> TemporalDirection {
//             if when == 0u64 {
//                 return TemporalDirection::Past;
//             }

//             TemporalDirection::Future
//         }
//     }

// 	fn test_timelock_macro() {
//         timelock!
//     }
// }
