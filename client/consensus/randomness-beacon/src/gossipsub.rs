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

use futures::{FutureExt, StreamExt};
use libp2p::{
	gossipsub,
	gossipsub::{
		Behaviour as GossipsubBehaviour, Config as GossipsubConfig,
		ConfigBuilder as GossipsubConfigBuilder, Event as GossipsubEvent, IdentTopic,
		Message as GossipsubMessage, MessageAuthenticity, MessageId, PublishError,
		SubscriptionError, Topic, TopicHash,
	},
	identity::Keypair,
	swarm::{NetworkBehaviour, Swarm, SwarmBuilder, SwarmEvent},
	tcp::Config,
	Transport,
};
use prost::Message;
use sp_consensus_randomness_beacon::types::{OpaquePulse, Pulse};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct GossipsubState {
	pub pulses: Vec<OpaquePulse>,
}

// Q: What about storage? Should we try to achieve perpetual storage of all observed data?
// that could be possible if we made each node a replica of some of the data/incentivized them to store pieces
// or should we just have a couple complete replica nodes and anyone else can have limited history?
impl GossipsubState {
	fn add_pulse(&mut self, pulse: Pulse) {
		self.pulses.push(pulse.into_opaque())
	}
}

pub type SharedState = Arc<Mutex<GossipsubState>>;

pub struct GossipsubNetwork {
	swarm: Swarm<GossipsubBehaviour>,
	state: SharedState,
}

impl GossipsubNetwork {
	pub fn new(
		local_key: &Keypair,
		state: SharedState,
	) -> Result<Self, Box<dyn std::error::Error>> {
		// Set the message authenticity - How we expect to publish messages
		// Here we expect the publisher to sign the message with their key.
		let message_authenticity = MessageAuthenticity::Signed(local_key.clone());
		// Create the Swarm
		// Create the transport with TCP
		let transport = libp2p::tcp::tokio::Transport::new(Config::default())
			.upgrade(libp2p::core::upgrade::Version::V1)
			.authenticate(libp2p::noise::Config::new(local_key)?)
			.multiplex(libp2p::yamux::Config::default())
			.boxed();

		// set default parameters for gossipsub
		let gossipsub_config = GossipsubConfig::default();
		// build a gossipsub network behaviour
		let mut gossipsub =
			GossipsubBehaviour::new(message_authenticity, gossipsub_config).unwrap();

		let mut swarm =
			SwarmBuilder::without_executor(transport, gossipsub, local_key.public().to_peer_id())
				.build();
		// dig TXT _dnsaddr.api.drand.sh
		let maddr1: libp2p::Multiaddr =
			"/ip4/184.72.27.233/tcp/44544/p2p/12D3KooWBhAkxEn3XE7QanogjGrhyKBMC5GeM3JUTqz54HqS6VHG"
				.parse()
				.unwrap();
		let maddr2: libp2p::Multiaddr = "/ip4/54.193.191.250/tcp/44544/p2p/12D3KooWQqDi3D3KLfDjWATQUUE4o5aSshwBFi9JM36wqEPMPD5y".parse().unwrap();
		// TODO: retry logic & reconnect if dropped logic
		swarm.dial(maddr1)?;
		swarm.dial(maddr2)?;
		Ok(Self { swarm, state })
	}

	/// Create a subscription to a gossipsub topic
	/// It writes new messages to the SharedState whenever received
	pub async fn subscribe(&mut self, topic_str: &str) -> Result<(), Box<dyn std::error::Error>> {
		// Start listening on a random port
		self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

		log::info!("Subscribing to gossipsub topic: {:?}", topic_str);
		let topic = IdentTopic::new(topic_str);
		self.swarm.behaviour_mut().subscribe(&topic)?;

		// NOTE: If there are no messages, then we shouldn't bother trying to run the inherent
		loop {
			match self.swarm.next().await {
				Some(SwarmEvent::Behaviour(gossipsub::Event::Message {
					propagation_source,
					message_id,
					message,
				})) => {
					let res = Pulse::decode(&*message.data).unwrap();
					let mut state = self.state.lock().unwrap();
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
					log::info!("Other: {:?}", x);
				},
				_ => {},
			}
		}
	}

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
