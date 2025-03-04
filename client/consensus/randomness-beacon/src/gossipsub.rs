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

//! # Gossipsub Network Subscription Client
//!
//! The `GossipsubNetwork` is a libp2p node designed to ingest well-formatted messages
//! from a gossipsub topic. The implemention is intended to be used with
//! the Drand beacon gossipsub topic, to which `Pulse` messages are published as protobuf messages.
//!
//! ## Overview
//!
//! - runs a libp2p node and handles peer connections
//! - subscribes to a gossipsub topic and writes well-formed messages to a [`SharedState`]
//!
//! ## Examples
//!
//! ``` no_run
//! use sc_consensus_randomness_beacon::gossipsub::GossipsubNetwork;
//! use sc_consensus_randomness_beacon::types::*;
//! use futures::StreamExt;
//! use libp2p::{
//! 		gossipsub,
//! 		gossipsub::{
//! 			Behaviour as GossipsubBehaviour, Config as GossipsubConfig, IdentTopic, MessageAuthenticity,
//! 		},
//! 		identity::Keypair,
//! 		swarm::{Swarm, SwarmEvent},
//! 		Multiaddr, SwarmBuilder,
//! };
//! use sc_utils::mpsc::tracing_unbounded;
//! use prost::Message;
//! use std::sync::{Arc, Mutex};
//!
//! let topic_str: &str =
//! 	"/drand/pubsub/v0.0.0/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971";
//! let maddr1: Multiaddr =
//! 	"/ip4/184.72.27.233/tcp/44544/p2p/12D3KooWBhAkxEn3XE7QanogjGrhyKBMC5GeM3JUTqz54HqS6VHG"
//! 		.parse()
//! 		.expect("The string is a well-formatted multiaddress. qed.");
//! let maddr2: Multiaddr =
//! 	"/ip4/54.193.191.250/tcp/44544/p2p/12D3KooWQqDi3D3KLfDjWATQUUE4o5aSshwBFi9JM36wqEPMPD5y"
//! 		.parse()
//! 		.expect("The string is a well-formatted multiaddress. qed.");
//! let local_identity: Keypair = Keypair::generate_ed25519();
//! let (tx, rx) = tracing_unbounded("drand-notification-channel", 100000);
//! let gossipsub_config = GossipsubConfig::default();
//! let mut gossipsub = GossipsubNetwork::new(&local_identity, gossipsub_config, tx, None).unwrap();
//! tokio::spawn(async move {
//! 	if let Err(e) = gossipsub.run(topic_str, vec![maddr1, maddr2]).await {
//! 		log::error!("Failed to run gossipsub network: {:?}", e);
//! 	}
//! });
//! ```
use crate::types::*;
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
use sc_utils::mpsc::TracingUnboundedSender;

/// The default address instructing libp2p to choose a random open port on the local machine
const RAND_LISTEN_ADDR: &str = "/ip4/0.0.0.0/tcp/0";

/// Various errors that can be encountered
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
	/// The signature buffer expects 48 bytes, but more were provided
	SignatureBufferCapacityExceeded,
	/// The message did not follow the expected format
	UnexpectedMessageFormat,
	/// The provided gossipsub behaviour is invalid
	InvalidGossipsubNetworkBehaviour,
	/// The multiaddress is invalid (likely the protocol is not supported)
	InvalidMultiaddress { who: Multiaddr },
	/// The swarm could not listen on the given port
	SwarmListenFailure,
}

/// A gossipsub network with any behaviour and shared state
pub struct GossipsubNetwork {
	/// The behaviour config for the swam
	swarm: Swarm<GossipsubBehaviour>,
	/// The mpsc channel sender
	sender: TracingUnboundedSender<OpaquePulse>,
	/// The number of peers the node is connected to
	pub(crate) connected_peers: u8,
}

impl GossipsubNetwork {
	/// Build a new gossipsub network.
	/// It constructs a libp2p [swarm](https://docs.rs/libp2p/latest/libp2p/struct.Swarm.html)
	/// where message authenticity requires signatures from the provided key and with a tcp-based
	/// transport layer.
	///
	/// * `key`: A libp2p keypair
	/// * `gossipsub_config`: A gossipsub config
	/// * `sender`: A `TracingUnboundedSender` that can send an `OpaquePulse`
	/// * `listen_addr`: An optional address to listen on. If None, then a random local port will be assigned.
	pub fn new(
		key: &Keypair,
		gossipsub_config: GossipsubConfig,
		sender: TracingUnboundedSender<OpaquePulse>,
		listen_addr: Option<&Multiaddr>,
	) -> Result<Self, Error> {
		let message_authenticity = MessageAuthenticity::Signed(key.clone());
		let gossipsub = GossipsubBehaviour::new(message_authenticity, gossipsub_config)
			.map_err(|_| Error::InvalidGossipsubNetworkBehaviour)?;
		// setup a libp2p swarm with tcp transport, using noise protocol for encryption
		// and yamux for multiplexing
		let mut swarm = SwarmBuilder::with_existing_identity(key.clone())
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

		// fallback to a randomly assigned open port if one was not provided
		let fallback = &RAND_LISTEN_ADDR.parse().expect("The multiaddress is well-formatted;QED.");
		let listen_addr = listen_addr.unwrap_or(fallback);

		swarm.listen_on(listen_addr.clone()).map_err(|_| Error::SwarmListenFailure)?;

		Ok(Self { swarm, sender, connected_peers: 0 })
	}

	/// Start the gossipsub network.
	/// It waits for peers to establish a connection, then writes well-formed messages received
	/// from the gossipsub topic to the shared state.
	///
	/// * `topic_str`: The gossipsub topic to subscribe to.
	/// * `peers`: A list of peers to dial.
	pub async fn run(&mut self, topic_str: &str, peers: Vec<Multiaddr>) -> Result<(), Error> {
		if !peers.is_empty() {
			for peer in &peers {
				self.swarm.dial((*peer).clone()).map_err(|_| {
					return Error::InvalidMultiaddress { who: (*peer).clone() };
				})?;
			}
			self.wait_for_peers(peers.len()).await;
		}

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
		self.connected_peers = connected_peers as u8;
	}

	/// Create a subscription to a gossipsub topic.
	/// It writes new messages to the SharedState whenever they are decodable as Pulses
	/// and ignores and messages it cannot understand.
	///
	/// * `topic_str`: The gossipsub topic to subscribe to.
	async fn subscribe(&mut self, topic_str: &str) -> Result<(), Error> {
		let topic = IdentTopic::new(topic_str);
		// *SRLabs: The error can never be encountered
		// Q: Can we use an expect, or is this unsafe?
		// Ref: https://docs.rs/libpp-gossipsub/0.48.0/src/libp2p_gossipsub/behaviour.rs.html#532
		// The error can only occur if the subscription filter rejects it, but we specify no filter.
		self.swarm
			.behaviour_mut()
			.subscribe(&topic)
			.expect("The libp2p gossipsub behavior has no subscription filter.");

		loop {
			match self.swarm.next().await {
				Some(SwarmEvent::Behaviour(gossipsub::Event::Message { message, .. })) => {
					match try_handle_pulse(&message.data) {
						Ok(pulse) => {
							self.sender.unbounded_send(pulse.clone()).unwrap();
						},
						Err(_) => {
							// handle non-decodable messages: https://github.com/ideal-lab5/idn-sdk/issues/60
						},
					}
				},
				_ => {
					// ignore all other events
				},
			}
		}
	}
}

pub(crate) fn try_handle_pulse(data: &[u8]) -> Result<OpaquePulse, Error> {
	let pulse = Pulse::decode(data).map_err(|_| Error::UnexpectedMessageFormat)?;
	let pulse: OpaquePulse =
		pulse.try_into().map_err(|_| Error::SignatureBufferCapacityExceeded)?;

	Ok(pulse)
}

#[cfg(test)]
mod tests {
	use super::*;
	use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver};
	use tokio::time::{sleep, Duration};

	#[test]
	fn can_convert_valid_data_to_opaque_pulse() {
		let pulse = Pulse {
			round: 14475418,
			signature: [
				146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242, 43, 61, 28, 75, 93, 37,
				95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104, 76, 235, 84, 49,
				223, 95, 22, 186, 113, 163, 202, 195, 230, 117,
			]
			.to_vec(),
		};
		let opaque: OpaquePulse = pulse.clone().try_into().unwrap();
		let mut data = Vec::new();
		pulse.encode(&mut data).unwrap();

		let actual_opaque = try_handle_pulse(&data).unwrap();
		assert_eq!(opaque, actual_opaque, "The output should match the input");
	}

	#[test]
	fn can_fail_when_data_not_decodable_to_pulse() {
		let res = try_handle_pulse(&[1; 32]);
		assert!(res.is_err());
		assert_eq!(
			res,
			Err(Error::UnexpectedMessageFormat),
			"There should be an `UnexpectedMessageFormat` error."
		);
	}

	#[test]
	fn can_fail_when_pulse_signature_exceeds_buffer() {
		let pulse = Pulse {
			round: 14475418,
			signature: [
				146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242, 43, 61, 28, 75, 93, 37,
				95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104, 76, 235, 84, 49,
				223, 95, 22, 186, 113, 163, 202, 195, 230, 117, 0, 0, 0, 0, 0, 0, 0, 0, 0,
			]
			.to_vec(),
		};

		let expected_error = Error::SignatureBufferCapacityExceeded;

		let mut data = Vec::new();
		pulse.encode(&mut data).unwrap();

		let res = try_handle_pulse(&data);
		assert!(res.is_err());
		assert_eq!(
			res,
			Err(expected_error),
			"There should be an `SignatureBufferCapacityExceeded` error."
		);
	}

	fn build_node() -> (GossipsubNetwork, TracingUnboundedReceiver<OpaquePulse>) {
		let local_identity: Keypair = Keypair::generate_ed25519();
		let gossipsub_config = GossipsubConfig::default();
		let (tx, rx) = tracing_unbounded("drand-notification-channel", 100000);
		(GossipsubNetwork::new(&local_identity, gossipsub_config, tx, None).unwrap(), rx)
	}

	#[tokio::test]
	async fn can_not_build_node_with_invalid_gossipsub_behavior() {
		// supply a signing key but set anon validation, an invalid config
		let local_identity: Keypair = Keypair::generate_ed25519();
		let gossipsub_config = libp2p::gossipsub::ConfigBuilder::default()
			.validation_mode(libp2p::gossipsub::ValidationMode::Anonymous)
			.build()
			.unwrap();
		let (tx, _rx) = tracing_unbounded("drand-notification-channel", 100000);
		let res = GossipsubNetwork::new(&local_identity, gossipsub_config, tx, None);
		assert!(res.is_err());
	}

	#[tokio::test]
	async fn can_build_new_node() {
		let (node, _rx) = build_node();
		assert!(node.connected_peers == 0, "There should be no connected peers.");
	}

	#[tokio::test]
	async fn can_build_new_node_with_listen_addr() {
		let local_identity: Keypair = Keypair::generate_ed25519();
		let gossipsub_config = GossipsubConfig::default();
		let (tx, _rx) = tracing_unbounded("drand-notification-channel", 100000);
		let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/4001".parse().unwrap();
		let node = GossipsubNetwork::new(&local_identity, gossipsub_config, tx, Some(&listen_addr))
			.unwrap();
		assert!(node.connected_peers == 0, "There should be no connected peers.");
	}

	#[tokio::test]
	async fn can_build_node_and_run_without_peers() {
		let topic_str = "test";
		let (mut node, _rx) = build_node();

		let mut is_err: bool = false;

		tokio::spawn(async move {
			if let Err(_e) = node.run(topic_str, vec![]).await {
				is_err = true;
			}
		});

		sleep(Duration::from_secs(1)).await;

		assert!(!is_err, "There should be no errors.");
	}

	#[tokio::test]
	async fn can_build_node_and_fail_with_random_peers() {
		let topic_str = "test";
		let (mut node, _rx) = build_node();

		let fake_peer: Multiaddr = Multiaddr::empty().with_p2p(libp2p::PeerId::random()).unwrap();

		let mut is_err: bool = false;

		tokio::spawn(async move {
			if let Err(_e) = node.run(topic_str, vec![fake_peer]).await {
				is_err = true;
			}
		});

		sleep(Duration::from_secs(2)).await;

		assert!(!is_err, "There should not be an error.");
	}

	#[tokio::test]
	async fn can_fail_when_bad_listen_addr_provided() {
		let fake_listen_addr: Multiaddr =
			"/ip4/127.0.0.2/tcp/1010/p2p/12D3KooWBhAkxEn3XE7QanogjGrhyKBMC5GeM3JUTqz54HqS6VHG"
				.parse()
				.unwrap();

		let local_identity: Keypair = Keypair::generate_ed25519();
		let gossipsub_config = GossipsubConfig::default();
		let (tx, _rx) = tracing_unbounded("drand-notification-channel", 100000);
		let res =
			GossipsubNetwork::new(&local_identity, gossipsub_config, tx, Some(&fake_listen_addr));
		assert!(res.is_err());
		assert!(matches!(res, Err(Error::SwarmListenFailure)), "Expected SwarmListenFailure error");
	}

	#[tokio::test]
	async fn test_gossipsub_network_listen_failure() {
		let key = Keypair::generate_ed25519();
		let (tx, _rx) = tracing_unbounded("drand-notification-channel", 100000);
		let config = GossipsubConfig::default();
		let invalid_addr: Multiaddr = Multiaddr::empty();

		let result = GossipsubNetwork::new(&key, config, tx, Some(&invalid_addr));
		assert!(result.is_err(), "Expected failure due to invalid listen address");
	}

	#[cfg(feature = "e2e ")]
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

		let (mut gossipsub, mut rx) = build_node();

		tokio::spawn(async move {
			if let Err(e) = gossipsub.run(topic_str, vec![maddr1, maddr2]).await {
				log::error!("Failed to run gossipsub network: {:?}", e);
			}
		});

		// Sleep for 6 secs
		sleep(Duration::from_millis(7000)).await;
		let p1 = rx.next().await;
		let p2 = rx.next().await;
		assert!(p2.unwrap().round == p1.unwrap().round + 1, "rounds should be incremental.");
	}
}
