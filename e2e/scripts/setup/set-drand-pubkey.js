const DRAND_PUBKEY = "0x83cf0f2896adee7eb8b5f01fcad3912212c437e0073e911fb90022d3e760183c8c4b450b6a0a6c3ac6a5776a2d1064510d1fec758c921cc22b0e17e63aaf4bcb5ed66304de9cf809bd274ca73bab4af5a6e9c76a4bc09e76eae8991ef5ece45a"


async function run(nodeName, networkInfo, args) {
    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    await zombie.util.cryptoWaitReady();
    
    const keyring = new zombie.Keyring({ type: "sr25519" });
    const alice = keyring.addFromUri("//Alice");
    const sudoPair = keyring.getPair(alice.publicKey);

    const unsub = await api.tx.sudo.sudo(api.tx.randBeacon.setBeaconConfig(DRAND_PUBKEY)).signAndSend(sudoPair, (result)=>{
        if (result.status.isInBlock) {
            console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
          } else if (result.status.isFinalized) {
            console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
            console.log('DRAND Pubkey Set')
            unsub(); // stop listening
          } else if (result.isError) {
            unsub();
          }
    });
}

module.exports = {run}