async function run(nodeName, networkInfo, args) {
  const { wsUri, userDefinedTypes } = networkInfo.nodesByName[nodeName];
  const api = await zombie.connect(wsUri, userDefinedTypes);

  await zombie.util.cryptoWaitReady();

  const keyring = new zombie.Keyring({ type: "sr25519" });
  const alice = keyring.addFromUri("//Alice");
  const sudoPair = keyring.getPair(alice.publicKey);

  // given X credits, we aim to verify the amount of DOT paid over the sub's lifetime
  const credits = BigInt(args[0]);
  const frequency = 1;
  const metadata = null;
  // Create a new Uint32Array with a specified length (e.g., 10 elements)
  const subId = new Uint8Array(33);
  // Fill the array with cryptographically strong random values
  // The crypto.getRandomValues() method fills the provided typed array with random numbers.
  // Math.floor(Math.random() * max);
  crypto.getRandomValues(subId);

  const unsub = await api.tx.sudo.sudo(api.tx.idnConsumer.sudoCreateSubscription(credits, frequency, metadata, subId))
    .signAndSend(sudoPair, (result) => {
      if (result.status.isInBlock) {
        console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
      } else if (result.status.isFinalized) {
        console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
        console.log("Subscription created successfully")
        unsub(); // stop listening
      } else if (result.isError) {
        unsub();
      }
    });
}

module.exports = { run }