async function run(nodeName, networkInfo, args) {
  const { wsUri, userDefinedTypes } = networkInfo.nodesByName[nodeName];
  const api = await zombie.connect(wsUri, userDefinedTypes);

  await zombie.util.cryptoWaitReady();

  const keyring = new zombie.Keyring({ type: "sr25519" });
  const alice = keyring.addFromUri("//Alice");
  const sudoPair = keyring.getPair(alice.publicKey);

  // given X credits, we aim to verify the amount of DOT paid over the sub's lifetime
  const credits = args[0];
  // frequency is ignored (!frequency-aware)
  const frequency = 0;
  const metadata = null;
  const subId = args[1];

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