
async function run(nodeName, networkInfo, args) {

    const account = args[0]
    const deposit = BigInt(args[1])


    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    await zombie.util.cryptoWaitReady();
    
    const keyring = new zombie.Keyring({ type: "sr25519" });
    const alice = keyring.addFromUri("//Alice");
    const keyPair = keyring.getPair(alice.publicKey);
    const tx = api.tx.balances.transferKeepAlive(account, deposit);
    await waitForFinalization(tx, keyPair);
    
}

async function waitForFinalization(tx, keyPair) {
    return new Promise(async (resolve, reject) => {
        const unsub = await tx.signAndSend(keyPair, (result) => {
          if (result.status.isInBlock) {
            console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
          } else if (result.status.isFinalized) {
            console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
            unsub(); // stop listening
            resolve(result); // resolve the Promise
          } else if (result.isError) {
            unsub();
            reject(new Error("Transaction failed"));
          }
    });
  });

}

module.exports = {run}