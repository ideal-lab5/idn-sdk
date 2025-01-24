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

/// The Gossipsub State tracks the pulses ingested from a randomness beacon
/// during some given block's lifetime (i.e. the new pulses observed)
#[derive(Debug, Clone)]
pub struct GossipsubState {
	/// An unbounded vec of pulses
	pub pulses: Vec<OpaquePulse>,
}

impl GossipsubState {
	/// append a new pulse to the pulses vec
	fn add_pulse(&mut self, pulse: Pulse) {
		self.pulses.push(pulse.into_opaque())
	}
}

/// Various errors that can be encountered
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
	/// The provided gossipsub behaviour is invalid
	InvalidGossipsubNetworkBehaviour,
	/// The provided local key is invalid
	InvalidLocalKey,
	/// The message could not be decoded.
	NondecodableMessage,
	/// The peer could not be dialed.
	PeerUnreachable {
		who: Multiaddr,
	},
	/// The swarm could not listen on the given port
	SwarmListenFailure,
	GossipsubSubscriptionFailed,
	StateLocked,
	InvalidSwarmConfig,
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
	///
	/// * `local_key`: A local libp2p keypair
	/// * `peers`: A list of peers to dial.
	/// * `state`: A shared state
	/// * `listen_addr`: An (optional) address to attempt to listen on. If None, then it attempts to
	///   listen on a random port on localhost
	pub fn new(
		local_key: &Keypair,
		peers: Vec<&Multiaddr>,
		state: SharedState,
		listen_addr: Option<&Multiaddr>,
	) -> Result<Self, Error> {
		// Set the message authenticity - How we expect to publish messages
		// Here we expect the publisher to sign the message with their key.
		let message_authenticity = MessageAuthenticity::Signed(local_key.clone());
		// set default parameters for gossipsub
		let gossipsub_config = GossipsubConfig::default();
		// build a gossipsub network behaviour
		let gossipsub = GossipsubBehaviour::new(message_authenticity, gossipsub_config)
			.map_err(|_| Error::InvalidGossipsubNetworkBehaviour)?;
		let mut swarm = SwarmBuilder::with_existing_identity(local_key.clone())
			.with_tokio()
			.with_tcp(
				libp2p::tcp::Config::default(),
				libp2p::noise::Config::new,
				libp2p::yamux::Config::default,
			)
			.map_err(|_| Error::InvalidSwarmConfig)?
			.with_behaviour(|_| gossipsub)
			.map_err(|_| Error::InvalidSwarmConfig)?
			.build();

		// Start listening on a random port if one wasn't provided
		let random =
			&"/ip4/0.0.0.0/tcp/0".parse().expect("The multiaddress is well-formatted;QED.");
		let listen_addr = listen_addr.unwrap_or(random);
		swarm.listen_on(listen_addr.clone()).map_err(|_| Error::SwarmListenFailure)?;
		// dial peers
		for peer in peers {
			swarm
				.dial(peer.clone())
				.map_err(|_| Error::PeerUnreachable { who: peer.clone() })?;
		}

		Ok(Self { swarm, state })
	}

	/// Create a subscription to a gossipsub topic.
	/// It writes new messages to the SharedState whenever received.
	///
	/// * `topic_str`: The gossipsub topic to subscribe to
	pub async fn subscribe(&mut self, topic_str: &str) -> Result<(), Error> {
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
					let res =
						Pulse::decode(&*message.data).map_err(|_| Error::NondecodableMessage)?;
					let mut state = self.state.lock().map_err(|_| Error::StateLocked)?;
					state.add_pulse(res.clone());
					log::info!(
						"Received pulse for round {:?} with message id: {} from peer: {:?}",
						res.round,
						message_id,
						propagation_source
					);
				},
				Some(SwarmEvent::NewListenAddr { address, .. }) => {
					log::info!(" Listening on {:?}", address);
				},
				Some(x) => {
					log::info!("Unhandled status from libp2p: {:?}", x);
				},
				_ => {},
			}
		}
	}

	/// Publish a new message to a gossipsub topic
	/// Currently unused
	pub fn publish(
		&mut self,
		topic_str: &str,
		data: Vec<u8>,
	) -> Result<(), Box<dyn std::error::Error>> {
		let topic = IdentTopic::new(topic_str);
		self.swarm.behaviour_mut().publish(topic, data)?;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::{Arc, Mutex};

	fn build_node(
		peers: Vec<&Multiaddr>,
		listen_addr: Option<&Multiaddr>,
	) -> (GossipsubNetwork, SharedState) {
		let local_identity: Keypair = Keypair::generate_ed25519();
		let shared_state = Arc::new(Mutex::new(GossipsubState { pulses: vec![] }));
		(
			GossipsubNetwork::new(&local_identity, peers, shared_state.clone(), listen_addr)
				.unwrap(),
			shared_state,
		)
	}

	#[tokio::test]
	async fn can_create_new_gossipsub_network_with_empty_peers() {
		let (gossipsub, _s) = build_node(vec![], None);
		let data_lock = gossipsub.state.lock().unwrap();
		assert!(data_lock.pulses.is_empty());
	}

	#[test]
	fn can_add_pulse_to_state() {
		let mut state = GossipsubState { pulses: vec![] };
		let pulse = Pulse { round: 0, signature: ::prost::alloc::vec![1;48] };
		state.add_pulse(pulse);
		assert_eq!(state.pulses.len(), 1);
	}

	#[tokio::test]
	async fn can_not_publish_message_to_gossipsub_topic_when_no_peers() {
		let (mut gossipsub, _s) = build_node(vec![], None);
		let topic = "test-topic";
		let data = b"test-message".to_vec();
		let result = gossipsub.publish(topic, data);
		assert!(result.is_err());
	}
}
