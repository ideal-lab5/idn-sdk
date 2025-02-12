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

//! The `GossipsubNetwork` is a libp2p node designed to ingest well-formatted messages
//! from a gossipsub topic.More specifically, the implemention is intended to be used with
//! the Drand beacon gossipsub topic, to which `Pulse` messages are published.

use futures::StreamExt;
use libp2p::{
	gossipsub,
	gossipsub::{
		Behaviour as GossipsubBehaviour, Config as GossipsubConfig, IdentTopic, MessageAuthenticity,
	},
	identity::Keypair,
	swarm::{Swarm, SwarmEvent},
	Multiaddr, SwarmBuilder,
};
use prost::Message;
use sp_consensus_randomness_beacon::types::{OpaquePulse, Pulse};
use std::sync::{Arc, Mutex};

/// The default address instructing libp2p to choose a random open port on the local machine
const RAND_LISTEN_ADDR: &str = "/ip4/0.0.0.0/tcp/0";

/// Various errors that can be encountered
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
	/// The signature buffer expects 48 bytes, but more were provided
	SignatureBufferCapacityExceeded,
	/// The provided gossipsub behaviour is invalid
	InvalidGossipsubNetworkBehaviour,
	/// The peer could not be dialed.
	PeerUnreachable { who: Multiaddr },
	/// The swarm could not listen on the given port
	SwarmListenFailure,
	/// The swarm could not subscribe to the topic.
	GossipsubSubscriptionFailed,
	/// The Mutex is locked and can not be accessed.
	StateLocked,
}

/// The Gossipsub State tracks the pulses ingested from a randomness beacon
/// during some given block's lifetime (i.e. the new pulses observed)
#[derive(Debug, Clone, PartialEq)]
pub struct GossipsubState {
	/// An (unbounded) vec of pulses
	pub pulses: Vec<OpaquePulse>,
}

impl GossipsubState {
	/// Append a new pulse to the pulses vec. It returns Ok() if successful, otherwise gives an
	/// error.
	/// * `pulse`: The pulse to append
	fn add_pulse(&mut self, pulse: Pulse) -> Result<(), Error> {
		let opaque = pulse.try_into().map_err(|_| Error::SignatureBufferCapacityExceeded)?;
		self.pulses.push(opaque);
		Ok(())
	}
}

/// A shared Gossipsub state between threads
pub type SharedState = Arc<Mutex<GossipsubState>>;

/// A gossipsub network with any behaviour and shared state
pub struct GossipsubNetwork {
	/// The behaviour config for the swam
	swarm: Swarm<GossipsubBehaviour>,
	/// The shared state
	state: SharedState,
}

impl GossipsubNetwork {
	/// Build a new gossipsub network.
	/// It constructs a libp2p [swarm](https://docs.rs/libp2p/latest/libp2p/struct.Swarm.html)
	/// where message authenticity requires signatures from the provided key and with a tcp-based
	/// transport layer.
	///
	/// * `key`: A libp2p keypair
	/// * `state`: The shared state
	/// * `gossipsub_config`: A gossipsub config
	pub fn new(
		key: &Keypair,
		state: SharedState,
		gossipsub_config: GossipsubConfig,
	) -> Result<Self, Error> {
		let message_authenticity = MessageAuthenticity::Signed(key.clone());
		let gossipsub = GossipsubBehaviour::new(message_authenticity, gossipsub_config)
			.map_err(|_| Error::InvalidGossipsubNetworkBehaviour)?;
		let swarm = SwarmBuilder::with_existing_identity(key.clone())
			.with_tokio()
			.with_tcp(
				libp2p::tcp::Config::default(),
				libp2p::noise::Config::new,
				libp2p::yamux::Config::default,
			)
			.expect("The TCP config is correct.")
			.with_behaviour(|_| gossipsub)
			.expect("The behaviour is well defined.")
			.build();

		Ok(Self { swarm, state })
	}

	/// Start the gossipsub network.
	/// It waits for peers to establish a connection, then writes well-formed messages received
	/// from the gossipsub topic to the shared state.
	///
	/// * `topic_str`: The gossipsub topic to subscribe to
	/// * `peers`: A list of peers to dial
	/// * `listen_addr`: An address to listen on. If None, then a random local port is assigned.
	pub async fn run(
		&mut self,
		topic_str: &str,
		peers: Vec<&Multiaddr>,
		listen_addr: Option<&Multiaddr>,
	) -> Result<(), Error> {
		// fallback to a randomly assigned open port if one was not provided
		let fallback = &RAND_LISTEN_ADDR.parse().expect("The multiaddress is well-formatted;QED.");
		let listen_addr = listen_addr.unwrap_or(fallback);
		self.swarm
			.listen_on(listen_addr.clone())
			.map_err(|_| Error::SwarmListenFailure)?;

		for peer in &peers {
			self.swarm
				.dial((*peer).clone())
				.map_err(|_| Error::PeerUnreachable { who: (*peer).clone() })?;
		}

		self.wait_for_peers(peers.len()).await;
		self.subscribe(topic_str).await
	}

	/// Executes until at least `target_count` ConnectionEstablished events
	/// have been observed.
	/// * `target_count`: The number of connection established events to observe until it terminates
	async fn wait_for_peers(&mut self, target_count: usize) {
		let mut connected_peers = 0;
		while connected_peers < target_count {
			if let Some(SwarmEvent::ConnectionEstablished { .. }) = self.swarm.next().await {
				connected_peers += 1;
			}
		}
	}

	/// Create a subscription to a gossipsub topic.
	/// It writes new messages to the SharedState whenever they are decodable as Pulses
	/// and ignores and messages it cannot understand.
	///
	/// * `topic_str`: The gossipsub topic to subscribe to.
	async fn subscribe(&mut self, topic_str: &str) -> Result<(), Error> {
		let topic = IdentTopic::new(topic_str);

		self.swarm
			.behaviour_mut()
			.subscribe(&topic)
			.map_err(|_| Error::GossipsubSubscriptionFailed)?;

		loop {
			match self.swarm.next().await {
				Some(SwarmEvent::Behaviour(gossipsub::Event::Message {
					propagation_source,
					message_id,
					message,
				})) => {
					// ignore non-decodable messages
					if let Ok(pulse) = Pulse::decode(&*message.data) {
						// The state could be locked here: https://github.com/ideal-lab5/idn-sdk/issues/59
						if let Ok(mut state) = self.state.lock() {
							state.add_pulse(pulse.clone())?;
							log::info!(
								"Received pulse for round {:?} with message id: {} from peer: {:?}",
								pulse.round,
								message_id,
								propagation_source
							);
						}
					}
					// handle non-decodable messages: https://github.com/ideal-lab5/idn-sdk/issues/60
				},
				_ => {
					// ignore all other events
				},
			}
		}
	}
}

#[cfg(feature = "e2e")]
#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::{Arc, Mutex};
	use tokio::time::{sleep, Duration};

	fn build_node() -> (GossipsubNetwork, SharedState) {
		let local_identity: Keypair = Keypair::generate_ed25519();
		let state = Arc::new(Mutex::new(GossipsubState { pulses: vec![] }));
		let gossipsub_config = GossipsubConfig::default();
		(GossipsubNetwork::new(&local_identity, state.clone(), gossipsub_config).unwrap(), state)
	}

	#[tokio::test]
	async fn can_subscribe_to_topic_and_deserialize_pulses_when_peers_connected() {
		let topic_str: &str =
			"/drand/pubsub/v0.0.0/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971";

		let maddr1: libp2p::Multiaddr =
			"/ip4/184.72.27.233/tcp/44544/p2p/12D3KooWBhAkxEn3XE7QanogjGrhyKBMC5GeM3JUTqz54HqS6VHG"
				.parse()
				.expect("The string is a well-formatted multiaddress. qed.");

		let maddr2: libp2p::Multiaddr =
        "/ip4/54.193.191.250/tcp/44544/p2p/12D3KooWQqDi3D3KLfDjWATQUUE4o5aSshwBFi9JM36wqEPMPD5y"
            .parse()
            .expect("The string is a well-formatted multiaddress. qed.");

		let (mut gossipsub, state) = build_node();
		tokio::spawn(async move {
			if let Err(e) = gossipsub.run(topic_str, vec![&maddr1, &maddr2], None).await {
				log::error!("Failed to run gossipsub network: {:?}", e);
			}
		});

		// Sleep for 6 secs
		sleep(Duration::from_millis(6000)).await;
		let data_lock = state.lock().unwrap();
		let pulses = &data_lock.pulses;
		assert!(pulses.len() >= 1);
	}
}

#[cfg(not(feature = "e2e"))]
#[cfg(test)]
mod tests {
	use super::*;
	use libp2p::gossipsub::{ConfigBuilder, ValidationMode};
	use std::sync::{Arc, Mutex};
	use tokio::time::{sleep, Duration};

	fn build_node() -> (GossipsubNetwork, SharedState) {
		let local_identity: Keypair = Keypair::generate_ed25519();
		let state = Arc::new(Mutex::new(GossipsubState { pulses: vec![] }));
		let gossipsub_config = GossipsubConfig::default();
		(GossipsubNetwork::new(&local_identity, state.clone(), gossipsub_config).unwrap(), state)
	}

	#[test]
	fn can_add_pulse_to_state() {
		let mut state = GossipsubState { pulses: vec![] };
		let pulse = Pulse { round: 0, signature: ::prost::alloc::vec![1;48] };
		let res = state.add_pulse(pulse);
		assert!(res.is_ok());
		assert_eq!(state.pulses.len(), 1);
	}

	#[test]
	fn can_not_pulse_to_state_if_sig_buff_too_big() {
		let mut state = GossipsubState { pulses: vec![] };
		let pulse = Pulse { round: 0, signature: ::prost::alloc::vec![1;49] };
		let res = state.add_pulse(pulse);
		assert!(res.is_err());
		assert_eq!(state.pulses.len(), 0);
	}

	#[tokio::test]
	async fn can_fail_on_invalid_gossipsub_network_behaviour() {
		let local_identity: Keypair = Keypair::generate_ed25519();
		let state = Arc::new(Mutex::new(GossipsubState { pulses: vec![] }));

		let gossipsub_config = ConfigBuilder::default()
			.validation_mode(ValidationMode::Anonymous)
			.build()
			.unwrap();

		let result = GossipsubNetwork::new(&local_identity, state, gossipsub_config);

		assert!(result.is_err());
		assert!(
			matches!(result, Err(Error::InvalidGossipsubNetworkBehaviour)),
			"Expected InvalidGossipsubNetworkBehaviour error"
		);
	}

	#[tokio::test]
	async fn can_create_new_gossipsub_network() {
		let (_gossipsub, state) = build_node();
		let data_lock = state.lock().unwrap();
		assert!(data_lock.pulses.is_empty());
	}

	#[tokio::test]
	async fn can_fail_when_bad_listen_addr_provided() {
		let fake_listen_addr: Multiaddr =
			"/ip4/127.0.0.2/tcp/1010/p2p/12D3KooWBhAkxEn3XE7QanogjGrhyKBMC5GeM3JUTqz54HqS6VHG"
				.parse()
				.unwrap();

		let (mut gossipsub, _s) = build_node();

		let res = gossipsub.run("test", vec![], Some(&fake_listen_addr)).await;
		assert!(res.is_err());
		assert!(matches!(res, Err(Error::SwarmListenFailure)), "Expected SwarmListenFailure error");
	}

	// #[tokio::test]
	// async fn can_fail_to_add_pulses_when_state_locked() {
	// 	let topic_str: &str =
	// 		"/drand/pubsub/v0.0.0/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971";

	// 	let maddr1: libp2p::Multiaddr =
	// 		"/ip4/184.72.27.233/tcp/44544/p2p/12D3KooWBhAkxEn3XE7QanogjGrhyKBMC5GeM3JUTqz54HqS6VHG"
	// 			.parse()
	// 			.expect("The string is a well-formatted multiaddress. qed.");

	// 	let maddr2: libp2p::Multiaddr =
    //     "/ip4/54.193.191.250/tcp/44544/p2p/12D3KooWQqDi3D3KLfDjWATQUUE4o5aSshwBFi9JM36wqEPMPD5y"
    //         .parse()
    //         .expect("The string is a well-formatted multiaddress. qed.");

	// 	let (mut gossipsub, state) = build_node();
	// 	// prematurely lock the state
	// 	let data_lock = state.lock().unwrap();

	// 	tokio::spawn(async move {
	// 		if let Err(e) = gossipsub.run(topic_str, vec![&maddr1, &maddr2], None).await {
	// 			log::error!("Failed to run gossipsub network: {:?}", e);
	// 		}
	// 	});

	// 	// Sleep for 6 secs
	// 	sleep(Duration::from_millis(6000)).await;
	// 	let pulses = &data_lock.pulses;
	// 	assert!(pulses.len() == 0);
	// }

	#[tokio::test]
	async fn can_fail_to_subscribe_to_topic_with_no_peers() {
		let topic_str: &str =
			"/drand/pubsub/v0.0.0/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971";

		let (mut gossipsub, state) = build_node();
		tokio::spawn(async move {
			if let Err(e) = gossipsub.run(topic_str, vec![], None).await {
				log::error!("Failed to run gossipsub network: {:?}", e);
			}
		});

		// Sleep for 6 secs
		sleep(Duration::from_millis(6000)).await;
		let data_lock = state.lock().unwrap();
		let pulses = &data_lock.pulses;
		assert!(pulses.len() == 0);
	}
}
