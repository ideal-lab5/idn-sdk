async function run(nodeName, networkInfo, args) {
    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    await zombie.util.cryptoWaitReady();

    const keyring = new zombie.Keyring({ type: "sr25519" });
    const alice = keyring.addFromUri("//Alice");
    const sudoPair = keyring.getPair(alice.publicKey);

    const subId = args[0];
    const reqRef = null;
    const originKind = "xcm"

    const unsub = await api.tx.idnConsumer.requestSubInfo(subId, reqRef, originKind).signAndSend(sudoPair, (result)=>{
        if (result.status.isInBlock) {
            console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
          } else if (result.status.isFinalized) {
            console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
            console.log("Subscription info request succeeded")
            unsub(); // stop listening
          } else if (result.isError) {
            unsub();
          }
    });
}

module.exports = {run}