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

use alloc::vec::Vec;
use codec::{Decode, Encode};

sp_api::decl_runtime_apis! {
    pub trait ExtrinsicBuilderApi<AccountId, RuntimeCall, Signature, TxExtension, Nonce> 
	where
		AccountId: Encode + Decode,
		RuntimeCall: Encode + Decode,
		Signature: Encode + Decode,
		TxExtension : Encode + Decode,
		Nonce: Encode + Decode,
	{
        fn construct_pulse_payload(
            asig: Signature,
            start: u64,
            end: u64,
            nonce: Nonce,
        ) -> (Vec<u8>, RuntimeCall, TxExtension);
    }
}
