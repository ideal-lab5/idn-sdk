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
//! the Drand beacon gossipsub topic, to which `ProtoPulse` messages are published as protobuf
//! messages.
//!
//! The `RetryableGossipsubRunner` is an infallible runner for the GossipsubNetwork.  
//! It constructs the gossipsub config and fresh keypair and uses it to spawn a new swarm and poll
//! events. Sometimes when syncing to the gossipsub mesh, peers fail to include our new identity
//! even though we can ping them. This would normally result in being able to ping peers, but never
//! recevie a message from the gossipsub topic. To account for this, the struct will tear down and
//! rebuild the swarm if we go `N` seconds without receiving a message. Note: This is *specifically*
//! intended for scenarios where the gossipsub topic is published to with regular periodicity.
//!
//! ## Overview
//!
//! The `GossipsubNetwork:
//! - runs a libp2p node and handles peer connections
//! - subscribes to a gossipsub topic and writes well-formed messages to a `SharedState`
//!
//! ## Examples
//!
//! ### Standalone GossipsubNetwork
//! The GossipsubNetwork struct can be used directly to ingest from the given topic
//! ``` no_run
//! use sc_consensus_randomness_beacon::gossipsub::GossipsubNetwork;
//! use sp_consensus_randomness_beacon::types::*;
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
//! 	if let Err(e) = gossipsub.run(topic_str, &[maddr1, maddr2]).await {
//! 		log::error!("Failed to run gossipsub network: {:?}", e);
//! 	}
//! });
//! ```
//!
//! ### RetryableGossipsubRunner
//!
//! ``` no_run
//! use sc_consensus_randomness_beacon::gossipsub::{DrandReceiver, GossipsubNetwork, RetryableGossipsubRunner};
//! use sp_consensus_randomness_beacon::types::*;
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
//! 	const MAX_CACHE_SIZE: usize = 30;
//! 	const NO_MESSAGE_TIMEOUT_SECS: usize = 12;
//!
//! 	// setup gossipsub network if you are an authority
//! 	let (tx, rx) = tracing_unbounded("drand-notification-channel", 10000);
//! 	// 30 pulses in storage at most
//! 	let drand_receiver = DrandReceiver::<MAX_CACHE_SIZE>::new(rx);
//! 	let topic_str: &str =
//! 		"/drand/pubsub/v0.0.0/52db9ba70e0cc0f6eaf7803dd07447a1f5477735fd3f661792ba94600c84e971";
//! 	let maddr1: Multiaddr =
//! 		"/ip4/184.72.27.233/tcp/44544/p2p/12D3KooWBhAkxEn3XE7QanogjGrhyKBMC5GeM3JUTqz54HqS6VHG"
//! 			.parse()
//! 			.expect("The string is a well-formatted multiaddress. qed.");
//! 	let maddr2: Multiaddr =
//! 		"/ip4/54.193.191.250/tcp/44544/p2p/12D3KooWQqDi3D3KLfDjWATQUUE4o5aSshwBFi9JM36wqEPMPD5y"
//! 			.parse()
//! 			.expect("The string is a well-formatted multiaddress. qed.");

//!	let peers = vec![maddr1, maddr2];
//!	RetryableGossipsubRunner::<MAX_CACHE_SIZE, NO_MESSAGE_TIMEOUT_SECS>::run(
//!		topic_str,
//!		peers,
//!		tx.clone(),
//!		drand_receiver.clone(),
//!	);
//! ```
//!
use ::futures::StreamExt;
use alloc::collections::VecDeque;
use libp2p::{
	gossipsub,
	gossipsub::{
		Behaviour as GossipsubBehaviour, Config as GossipsubConfig, ConfigBuilder,
		Event as GossipsubEvent, IdentTopic, MessageAuthenticity,
	},
	identity::Keypair,
	multiaddr::Protocol,
	ping::{Behaviour as PingBehaviour, Config as PingConfig, Event as PingEvent},
	swarm::{NetworkBehaviour, Swarm, SwarmEvent},
	Multiaddr, PeerId, SwarmBuilder,
};
use prost::Message;
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use sp_consensus_randomness_beacon::types::*;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::Mutex;

#[cfg(not(test))]
const LOG_TARGET: &'static str = "gossipsub";
#[cfg(test)]
const LOG_TARGET: &'static str = module_path!();

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

/// Events that are traced
enum TracingEvent {
	/// A new pulse was consumed
	NewPulse,
	/// A message was received from a trusted peer but we do not know how to decode it
	NondecodableMessage,
	/// The peer failed to pong back
	PongFailed,
	/// Successful pinged a peer
	PingSuccess,
	/// A peer could not be dialed again after disconnecting
	RedialFailure,
	/// An untrusted peer sent a message
	UntrustedPeer,
	/// An error occurred while trying to send the decoded pulse to the mpsc channel
	UnboundedSendError,
}

impl TracingEvent {
	fn value(&self) -> &str {
		match *self {
			TracingEvent::NewPulse => "🎲 New pulse received and stored.",
			TracingEvent::NondecodableMessage => {
				"❓A message was received but we could not decode it."
			},
			TracingEvent::PongFailed => "💀 Peer failed to pong!",
			TracingEvent::PingSuccess => "🏓 Ping to peer succeeded.",
			TracingEvent::RedialFailure => "❌ Failed to redial peer.",
			TracingEvent::UnboundedSendError => "❌ Unable to send message to the queue.",
			TracingEvent::UntrustedPeer => "⛔ Message from untrusted peer.",
		}
	}
}

/// The RetryableGossipsubRunner allows us to safely execute the `GossipNetwork::run` function
/// such that we can recover in the case that the gossipsub mesh fails to include our peer or
/// rejects it.
///
/// M is the number of pulses to cache in the Drand receiver struct
/// N is the retry timeout for resyncing with the swarm
pub struct RetryableGossipsubRunner<const M: usize, const N: usize>;
impl<const M: usize, const N: usize> RetryableGossipsubRunner<M, N> {
	pub fn run(
		topic_str: &'static str,
		peers: Vec<Multiaddr>,
		tx: TracingUnboundedSender<CanonicalPulse>,
		rx: DrandReceiver<M>,
	) -> Result<(), Error> {
		tokio::spawn(async move {
			loop {
				// create a temp channel internally and forward it to the main channel
				// this lets use freely recreate the swarm + internal state without impacting outer
				// state
				let (tx_temp, mut rx_temp) = tracing_unbounded("drand-temp-channel", 10000);
				let tx_clone = tx.clone();
				tokio::spawn(async move {
					while let Some(pulse) = rx_temp.next().await {
						let _ = tx_clone.unbounded_send(pulse);
					}
				});

				let local_identity: Keypair = Keypair::generate_ed25519();
				let gossipsub_config = build_config();
				let mut gossipsub =
					GossipsubNetwork::new(&local_identity, gossipsub_config, tx_temp, None)
						.unwrap();
				// Spawn run task
				let peers = peers.clone();
				let run_handle = tokio::spawn(async move {
					if let Err(e) = gossipsub.run(topic_str, &peers).await {
						log::error!("Failed to run gossipsub network: {:?}", e);
					}
				});
				// sometimes the gossipsub mesh refuses to include our node on startup
				// as a result, we fail to ingest any messages from the topic even when they are
				// published in such a case we just reboot the node, which redials peers and
				// attempts entry to the mesh again
				let mut last_nonempty = tokio::time::Instant::now();
				loop {
					// the task should never complete, if it does we restart it
					if run_handle.is_finished() {
						log::warn!("gossipsub.run exited early — restarting.");
						break;
					}
					// Check if any new pulses are available
					if !rx.read().await.is_empty() {
						last_nonempty = tokio::time::Instant::now();
					}
					// restart if > N secs passed
					if last_nonempty.elapsed() > Duration::from_secs(N as u64) {
						tracing::info!(target: LOG_TARGET, "No Gossipsub messages in > {}s — restarting.", N);
						run_handle.abort();
						break;
					}
					tokio::time::sleep(Duration::from_millis(500)).await;
				}
			}
		});

		Ok(())
	}
}

/// receive messages from drand
#[derive(Clone)]
pub struct DrandReceiver<const N: usize> {
	/// A collection of received and unconsumed pulses
	pub pulses: Arc<Mutex<VecDeque<CanonicalPulse>>>,
}

impl<const N: usize> DrandReceiver<N> {
	/// Constructs a new DrandReceiver and starts receiving pulses
	///
	/// * `rx`: A [`TracingUnboundedReceiver`] to which [`sp_consensus_randomness_beacon::types::
	///   CanonicalPulse`] are written
	///  
	pub fn new(mut rx: TracingUnboundedReceiver<CanonicalPulse>) -> Self {
		let pulses = Arc::new(Mutex::new(VecDeque::new()));
		let pulses_clone = pulses.clone();
		// start a thread that writes new pulses to storage
		tokio::spawn(async move {
			while let Some(pulse) = rx.next().await {
				let mut locked_pulses = pulses_clone.lock().await;

				// Enforce FIFO with max size N
				if locked_pulses.len() == N {
					locked_pulses.pop_front();
				}
				locked_pulses.push_back(pulse);
			}
		});

		DrandReceiver { pulses }
	}

	/// Read the runtime pulses from storage
	pub async fn read(&self) -> Vec<CanonicalPulse> {
		let pulses = self.pulses.lock().await;
		let pulses = pulses.clone().into_iter().collect::<Vec<_>>();
		pulses.clone()
	}
}

/// A custom network behaviour that supports both gossipsub and ping protocols
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "ComposedEvent")]
struct CustomBehaviour {
	/// The gossipsub behaviour
	gossipsub: GossipsubBehaviour,
	/// The ping behaviour
	ping: PingBehaviour,
}

/// A custom event that composes gossipsub and ping events
#[derive(Debug)]
enum ComposedEvent {
	/// The gossipsub event
	Gossipsub(GossipsubEvent),
	/// The ping event
	Ping(PingEvent),
}

impl From<GossipsubEvent> for ComposedEvent {
	fn from(event: GossipsubEvent) -> Self {
		ComposedEvent::Gossipsub(event)
	}
}

impl From<PingEvent> for ComposedEvent {
	fn from(event: PingEvent) -> Self {
		ComposedEvent::Ping(event)
	}
}

/// A gossipsub network with any behaviour and shared state
pub struct GossipsubNetwork {
	/// The behaviour config for the swam
	swarm: Swarm<CustomBehaviour>,
	/// The mpsc channel sender
	sender: TracingUnboundedSender<CanonicalPulse>,
	// The list of allowed peer ids
	allowed_peer_map: HashMap<PeerId, Multiaddr>,
}

impl GossipsubNetwork {
	/// Build a new gossipsub network.
	/// It constructs a libp2p [swarm](https://docs.rs/libp2p/latest/libp2p/struct.Swarm.html)
	/// where message authenticity requires signatures from the provided key and with a tcp-based
	/// transport layer.
	///
	/// * `key`: A libp2p keypair
	/// * `gossipsub_config`: A gossipsub config
	/// * `sender`: A `TracingUnboundedSender` that can send an ` CanonicalPulse`
	/// * `listen_addr`: An optional address to listen on. If None, a random local port is assigned.
	pub fn new(
		key: &Keypair,
		gossipsub_config: GossipsubConfig,
		sender: TracingUnboundedSender<CanonicalPulse>,
		listen_addr: Option<&Multiaddr>,
	) -> Result<Self, Error> {
		let message_authenticity = MessageAuthenticity::Signed(key.clone());
		let gossipsub = GossipsubBehaviour::new(message_authenticity, gossipsub_config)
			.map_err(|_| Error::InvalidGossipsubNetworkBehaviour)?;
		// ping every 5 secs with 10s timeout
		let ping_config = PingConfig::new()
			.with_interval(Duration::from_secs(5))
			.with_timeout(Duration::from_secs(10));
		let ping = PingBehaviour::new(ping_config);
		let behaviour = CustomBehaviour { gossipsub, ping };
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
			.with_behaviour(|_| behaviour)
			.expect("The behaviour is well defined.")
			.build();

		// fallback to a randomly assigned open port if one was not provided
		let fallback = &RAND_LISTEN_ADDR.parse().expect("The multiaddress is well-formatted;QED.");
		let listen_addr = listen_addr.unwrap_or(fallback);

		swarm.listen_on(listen_addr.clone()).map_err(|_| Error::SwarmListenFailure)?;

		Ok(Self { swarm, sender, allowed_peer_map: HashMap::new() })
	}

	/// Start the gossipsub network.
	/// It waits for peers to establish a connection, then writes well-formed messages received
	/// from the gossipsub topic to the shared state.
	///
	/// * `topic_str`: The gossipsub topic to subscribe to.
	/// * `peers`: A list of peers to dial.
	pub async fn run(&mut self, topic_str: &str, peers: &[Multiaddr]) -> Result<(), Error> {
		let topic = IdentTopic::new(topic_str);
		// Ref: https://docs.rs/libpp-gossipsub/0.48.0/src/libp2p_gossipsub/behaviour.rs.html#532
		// The error can only occur if the subscription filter rejects it, but we specify no
		// filter. If this fails on startup, we consider it a critical failure and bring down the
		// node.
		let _ = self.swarm.behaviour_mut().gossipsub.subscribe(&topic).map_err(|_| {
			panic!("Subscription to the gossipsub topic {:?} failed. Stopping the node.", topic_str)
		});

		// dial peers
		for peer in peers {
			let peer_id = extract_peer_id(peer.clone());
			// if a peer id could be extracted, try to dial and add to allowed peer map
			if let Some(id) = peer_id {
				self.swarm.dial((*peer).clone()).map_err(|_| {
					return Error::InvalidMultiaddress { who: (*peer).clone() };
				})?;
				// add as an explicit peer to avoid ever being dropped, prevents mesh pruning of the
				// peer
				self.swarm.behaviour_mut().gossipsub.add_explicit_peer(&id);
				self.allowed_peer_map.insert(id, peer.clone());
			} else {
				// if we cannot extract a peer id then it is a critical failure
				return Err(Error::InvalidMultiaddress { who: (*peer).clone() });
			}
		}

		// gossipsub sometimes has issues when joining the mesh, causing it to fail to
		// propagate gossipsub Messages when polling the swarm (occurs roughly on <= 45% of
		// startups)

		self.handle_events().await
	}

	fn handle_event(&mut self, event: SwarmEvent<ComposedEvent>) {
		match &event {
			SwarmEvent::ConnectionEstablished { .. } => {
				log::info!(target: LOG_TARGET, "📡 connected to new peer!");
			},
			SwarmEvent::Behaviour(ComposedEvent::Gossipsub(gossipsub::Event::Message {
				message,
				propagation_source,
				..
			})) => {
				// reject messages authored by unknown peers
				if !self.allowed_peer_map.contains_key(&propagation_source) {
					tracing::warn!(target: LOG_TARGET, peer_id = %propagation_source, "{}", TracingEvent::UntrustedPeer.value());
				} else {
					match try_handle_pulse(&message.data) {
						Ok(pulse) => {
							tracing::info!(target: LOG_TARGET, round = %pulse.round, "{}", TracingEvent::NewPulse.value());
							if let Err(e) = self.sender.unbounded_send(pulse.clone()) {
								// Not sure how to test this line
								tracing::error!(target: LOG_TARGET, error = %e, "{}", TracingEvent::UnboundedSendError.value());
							}
						},
						Err(_) => {
							tracing::error!(target: LOG_TARGET, "{}", TracingEvent::NondecodableMessage.value());
						},
					}
				}
			},
			SwarmEvent::Behaviour(ComposedEvent::Ping(PingEvent { peer, result, .. })) => {
				match result {
					Ok(rtt) => {
						tracing::info!(target: LOG_TARGET, %peer, ?rtt, "{}", TracingEvent::PingSuccess.value());
					},
					Err(e) => {
						tracing::warn!(target: LOG_TARGET, error = %e, "{}", TracingEvent::PongFailed.value());
						// redial known peers
						if let Some(addr) = self.allowed_peer_map.get(&peer) {
							if let Err(e) = self.swarm.dial(addr.clone()) {
								tracing::error!(target: LOG_TARGET, %peer, ?e, "{}", TracingEvent::RedialFailure.value());
							}
						}
					},
				}
			},
			_ => {
				// ignore all other events
			},
		}
	}

	/// The main event loop for the libp2p node
	async fn handle_events(&mut self) -> Result<(), Error> {
		while let Some(event) = self.swarm.next().await {
			self.handle_event(event);
		}

		Ok(())
	}
}

/// Safely convert an encoded `ProtoPulse` to a `CanonicalPulse`
///
/// * `data`: The encoded `ProtoPulse`
pub(crate) fn try_handle_pulse(data: &[u8]) -> Result<CanonicalPulse, Error> {
	let pulse = ProtoPulse::decode(data).map_err(|_| Error::UnexpectedMessageFormat)?;
	let pulse: CanonicalPulse =
		pulse.try_into().map_err(|_| Error::SignatureBufferCapacityExceeded)?;

	Ok(pulse)
}

fn extract_peer_id(maddr: Multiaddr) -> Option<PeerId> {
	maddr
		.iter()
		.find_map(|p| if let Protocol::P2p(peer) = p { Some(peer) } else { None })
}

pub fn build_config() -> GossipsubConfig {
	ConfigBuilder::default()
		.heartbeat_interval(Duration::from_millis(1000))
		.duplicate_cache_time(Duration::from_secs(60))
		.mesh_n(6)
		.mesh_n_low(4)
		.mesh_n_high(12)
		.gossip_lazy(3)
		// note: there is a typo in this function name
		.gossip_retransimission(3)
		.build()
		.expect("Valid gossipsub configuration")
}

/// Helper functions for testing purposes only
#[cfg(test)]
impl GossipsubNetwork {
	fn set_allowed_peer_map(&mut self, allowed_peer_map: HashMap<PeerId, Multiaddr>) {
		self.allowed_peer_map = allowed_peer_map;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver};
	use tokio::time::{sleep, Duration};

	fn build_node() -> (GossipsubNetwork, TracingUnboundedReceiver<CanonicalPulse>) {
		let local_identity: Keypair = Keypair::generate_ed25519();
		let gossipsub_config = GossipsubConfig::default();
		let (tx, rx) = tracing_unbounded("drand-notification-channel", 100000);
		(GossipsubNetwork::new(&local_identity, gossipsub_config, tx, None).unwrap(), rx)
	}

	#[test]
	fn can_convert_valid_data_to_opaque_pulse() {
		let pulse = ProtoPulse {
			round: 14475418,
			signature: [
				146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242, 43, 61, 28, 75, 93, 37,
				95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104, 76, 235, 84, 49,
				223, 95, 22, 186, 113, 163, 202, 195, 230, 117,
			]
			.to_vec(),
		};
		let opaque: CanonicalPulse = pulse.clone().try_into().unwrap();
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
		let pulse = ProtoPulse {
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
		assert!(node.allowed_peer_map == HashMap::new(), "There should be no connected peers.");
	}

	#[tokio::test]
	async fn can_build_new_node_with_listen_addr() {
		let local_identity: Keypair = Keypair::generate_ed25519();
		let gossipsub_config = GossipsubConfig::default();
		let (tx, _rx) = tracing_unbounded("drand-notification-channel", 100000);
		let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/4001".parse().unwrap();
		let node = GossipsubNetwork::new(&local_identity, gossipsub_config, tx, Some(&listen_addr))
			.unwrap();
		assert!(node.allowed_peer_map == HashMap::new(), "There should be no connected peers.");
	}

	#[tokio::test]
	async fn can_build_node_and_run_without_peers() {
		let topic_str = "test";
		let (mut node, _rx) = build_node();

		tokio::spawn(async move {
			if let Err(_e) = node.run(topic_str, &[]).await {
				panic!("There should be no error");
			}
		});

		sleep(Duration::from_secs(1)).await;
		// if it did not panic, we are good
	}

	#[tokio::test]
	async fn can_build_node_and_dial_random_peers() {
		let topic_str = "test";
		let (mut node, _rx) = build_node();

		let fake_peer: Multiaddr = Multiaddr::empty().with_p2p(libp2p::PeerId::random()).unwrap();

		tokio::spawn(async move {
			if let Err(_e) = node.run(topic_str, &[fake_peer]).await {
				panic!("There should be no error");
			}
		});

		sleep(Duration::from_secs(2)).await;
		// if it did not panic, we are good
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

	#[tokio::test]
	#[tracing_test::traced_test]
	async fn can_reject_messages_from_untrusted_sources() {
		let (mut node, _rx) = build_node();

		let untrusted_peer_id = libp2p::PeerId::random();
		let message_id = gossipsub::MessageId::new(&[0]);
		let topic = IdentTopic::new("").hash();

		let message =
			gossipsub::Message { source: None, data: vec![], sequence_number: None, topic };

		let untrusted_gossipsub_event = ComposedEvent::Gossipsub(gossipsub::Event::Message {
			propagation_source: untrusted_peer_id,
			message_id,
			message,
		});

		node.handle_event(SwarmEvent::Behaviour(untrusted_gossipsub_event));

		assert!(logs_contain(TracingEvent::UntrustedPeer.value()));
		assert!(logs_contain(&untrusted_peer_id.to_string()));
	}

	#[tokio::test]
	#[tracing_test::traced_test]
	async fn can_accept_messages_from_trusted_sources_and_process_a_pulse() {
		let (mut node, _rx) = build_node();

		let trusted_peer_id = libp2p::PeerId::random();
		let maddr = Multiaddr::empty();
		let mut allow_map = HashMap::new();
		allow_map.insert(trusted_peer_id, maddr);
		node.set_allowed_peer_map(allow_map);

		let message_id = gossipsub::MessageId::new(&[0]);
		let topic = IdentTopic::new("").hash();

		let pulse = ProtoPulse {
			round: 14475418,
			signature: [
				146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242, 43, 61, 28, 75, 93, 37,
				95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104, 76, 235, 84, 49,
				223, 95, 22, 186, 113, 163, 202, 195, 230, 117,
			]
			.to_vec(),
		};

		let message = gossipsub::Message {
			source: None,
			data: pulse.encode_to_vec(),
			sequence_number: None,
			topic,
		};

		let trusted_gossipsub_event = ComposedEvent::Gossipsub(gossipsub::Event::Message {
			propagation_source: trusted_peer_id,
			message_id,
			message,
		});

		node.handle_event(SwarmEvent::Behaviour(trusted_gossipsub_event));

		assert!(logs_contain(TracingEvent::NewPulse.value()));
		assert!(logs_contain("14475418"));
	}

	#[tokio::test]
	#[tracing_test::traced_test]
	async fn can_accept_messages_from_trusted_sources_and_handle_unknown_message_formats() {
		let (mut node, _rx) = build_node();

		let trusted_peer_id = libp2p::PeerId::random();
		let maddr = Multiaddr::empty();
		let mut allow_map = HashMap::new();
		allow_map.insert(trusted_peer_id, maddr);
		node.set_allowed_peer_map(allow_map);

		let message_id = gossipsub::MessageId::new(&[0]);
		let topic = IdentTopic::new("").hash();

		let message =
			gossipsub::Message { source: None, data: vec![], sequence_number: None, topic };

		let trusted_gossipsub_event = ComposedEvent::Gossipsub(gossipsub::Event::Message {
			propagation_source: trusted_peer_id,
			message_id,
			message,
		});

		node.handle_event(SwarmEvent::Behaviour(trusted_gossipsub_event));
		assert!(logs_contain(TracingEvent::NondecodableMessage.value()));
	}

	#[tokio::test]
	#[tracing_test::traced_test]
	async fn can_ping_success() {
		let (mut node, _rx) = build_node();

		let trusted_peer_id = libp2p::PeerId::random();
		let maddr = Multiaddr::empty();
		let mut allow_map = HashMap::new();
		allow_map.insert(trusted_peer_id, maddr);
		node.set_allowed_peer_map(allow_map);

		let ping_success_event = ComposedEvent::Ping(PingEvent {
			peer: trusted_peer_id,
			connection: libp2p::swarm::ConnectionId::new_unchecked(1),
			result: Ok(Duration::from_secs(1)),
		});

		node.handle_event(SwarmEvent::Behaviour(ping_success_event));
		assert!(logs_contain(TracingEvent::PingSuccess.value()));
		assert!(logs_contain(&trusted_peer_id.to_string()));
	}

	#[tokio::test]
	#[tracing_test::traced_test]
	async fn can_handle_pong_failure() {
		let (mut node, _rx) = build_node();

		let trusted_peer_id = libp2p::PeerId::random();
		let maddr = Multiaddr::empty();
		let mut allow_map = HashMap::new();
		allow_map.insert(trusted_peer_id, maddr);
		node.set_allowed_peer_map(allow_map);

		let ping_success_event = ComposedEvent::Ping(PingEvent {
			peer: trusted_peer_id,
			connection: libp2p::swarm::ConnectionId::new_unchecked(1),
			result: Err(libp2p::ping::Failure::Unsupported),
		});

		node.handle_event(SwarmEvent::Behaviour(ping_success_event));
		assert!(logs_contain(TracingEvent::PongFailed.value()));
		assert!(logs_contain(&format!("{}", libp2p::ping::Failure::Unsupported)));
	}

	/* drand receiver tests */
	#[tokio::test]
	async fn test_can_build_new_drand_receiver() {
		let (tx, rx) = tracing_unbounded("test", 10000);

		let receiver = DrandReceiver::<10>::new(rx);

		let pulse = ProtoPulse {
			round: 14475418,
			signature: [
				146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242, 43, 61, 28, 75, 93, 37,
				95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104, 76, 235, 84, 49,
				223, 95, 22, 186, 113, 163, 202, 195, 230, 117,
			]
			.to_vec(),
		};
		let opaque: CanonicalPulse = pulse.clone().try_into().unwrap();
		// write an opaque pulse
		tx.unbounded_send(opaque.clone()).unwrap();

		sleep(Duration::from_secs(1)).await;

		let actual = receiver.read().await;
		assert_eq!(actual.len(), 1, "There should be one opaque pulse in the vec");
		assert_eq!(actual[0], opaque);
	}

	#[tokio::test]
	async fn test_can_prune_drand_receiver() {
		let (tx, rx) = tracing_unbounded("test", 10000);

		let receiver = DrandReceiver::<1>::new(rx);

		let pulse = ProtoPulse {
			round: 14475418,
			signature: [
				146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242, 43, 61, 28, 75, 93, 37,
				95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104, 76, 235, 84, 49,
				223, 95, 22, 186, 113, 163, 202, 195, 230, 117,
			]
			.to_vec(),
		};

		let pulse2 = ProtoPulse {
			round: 14475419,
			signature: [
				146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242, 43, 61, 28, 75, 93, 37,
				95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104, 76, 235, 84, 49,
				223, 95, 22, 186, 113, 163, 202, 195, 230, 117,
			]
			.to_vec(),
		};

		let opaque: CanonicalPulse = pulse.clone().try_into().unwrap();
		let opaque2: CanonicalPulse = pulse2.clone().try_into().unwrap();
		// write an opaque pulse
		tx.unbounded_send(opaque.clone()).unwrap();
		tx.unbounded_send(opaque2.clone()).unwrap();

		sleep(Duration::from_secs(1)).await;

		let actual = receiver.read().await;
		assert_eq!(actual.len(), 1, "There should be one opaque pulse in the vec");
		assert_eq!(actual[0], opaque2);
	}

	/*
		RetryableGossipsubRunner tests
	*/
	#[tokio::test]
	#[tracing_test::traced_test]
	async fn test_can_run_retryable_gossipsub_network_with_empty_peers() {
		const M: usize = 10;
		const N: usize = 1;
		let topic_str: &str = "test";
		let peers = vec![];
		let (tx, rx) = tracing_unbounded("test", 10000);
		let receiver = DrandReceiver::<M>::new(rx);
		assert!(RetryableGossipsubRunner::<M, N>::run(topic_str, peers, tx, receiver).is_ok());
	}

	#[tokio::test]
	#[tracing_test::traced_test]
	async fn test_retryable_runner_restarts_if_no_messages_arrive() {
		const M: usize = 5;
		const N: usize = 2;
		let topic_str: &str = "test";
		let peers = vec![];

		let (tx, rx) = tracing_unbounded("test", 100);
		let receiver = DrandReceiver::<M>::new(rx);
		let _ = RetryableGossipsubRunner::<M, N>::run(topic_str, peers, tx, receiver);
		// wait past the timeout
		tokio::time::sleep(Duration::from_secs(N as u64 + 2)).await;
		assert!(logs_contain("No Gossipsub messages in"));
		assert!(logs_contain("restarting"));
	}

	#[tokio::test]
	#[tracing_test::traced_test]
	async fn test_retryable_runner_no_restart_if_messages_arrive_in_time() {
		const M: usize = 5;
		const N: usize = 2;
		let topic_str: &str = "test";
		let peers = vec![];

		let (tx, rx) = tracing_unbounded("test", 100);
		let receiver = DrandReceiver::<M>::new(rx);
		let _ = RetryableGossipsubRunner::<M, N>::run(topic_str, peers, tx.clone(), receiver);

		tokio::time::sleep(Duration::from_secs(N as u64 - 1)).await;

		// send a pulse
		let pulse = ProtoPulse {
			round: 14475418,
			signature: [
				146, 37, 87, 193, 37, 144, 182, 61, 73, 122, 248, 242, 242, 43, 61, 28, 75, 93, 37,
				95, 131, 38, 3, 203, 216, 6, 213, 241, 244, 90, 162, 208, 90, 104, 76, 235, 84, 49,
				223, 95, 22, 186, 113, 163, 202, 195, 230, 117,
			]
			.to_vec(),
		};
		let opaque: CanonicalPulse = pulse.clone().try_into().unwrap();
		// write an opaque pulse
		tx.unbounded_send(opaque.clone()).unwrap();

		// wait past the timeout
		tokio::time::sleep(Duration::from_secs(N as u64 + 2)).await;
		// the logs should be clear of restart messages
		assert!(!logs_contain("No Gossipsub messages in"));
	}
}
