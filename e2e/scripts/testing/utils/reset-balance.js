async function run(nodeName, networkInfo, args) {
    const address = args[0]; // account to zero out
    const { wsUri, userDefinedTypes } = networkInfo.nodesByName[nodeName];

    const api = await zombie.connect(wsUri, userDefinedTypes);
    await zombie.util.cryptoWaitReady();
    const keyring = new zombie.Keyring({ type: "sr25519" });
    const alice = keyring.addFromUri("//Alice");
    const sudoKey = keyring.getPair(alice.publicKey);

    const ed = await api.consts.balances.existentialDeposit;
    // Sign and send transaction
    const unsub = await api.tx.sudo.sudo(
        api.tx.balances.forceSetBalance(address, ed + 1)
    ).signAndSend(sudoKey, (result) => {
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

    console.log(`Balance successfully reset to 0 for ${address}`);
}

module.exports = { run };