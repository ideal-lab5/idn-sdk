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

 // how would contracts use it?
 // let r = self.env().extension().fetch_random().unwrap();
 // // use r to randomly select from the list
 // would become
 // let list = ...
 // timelock!(1000000120);
 // let rand_list = select!(&self, list, k);
 // let shuffled = shuffle!(&self, list);

use ink::env::Environment;

pub fn select(env: &Environment, list: &[T], count: usize) -> &[T] {
    // 32 bytes
    let rand = self.env().extension().fetch_random().unwrap();
    
}

