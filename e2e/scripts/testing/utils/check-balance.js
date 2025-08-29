
async function run(nodeName, networkInfo, args) {

    const address = args[0]
    const amount = BigInt(args[1])
    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    await zombie.util.cryptoWaitReady();

    let bal  = await api.query.system.account(address);
    let b = BigInt(parseInt(bal.data.free))    
    if (b !== amount) {
        throw new Error("The treasury balance is: " + b + ", but should be: " + amount)
    }
    
}

module.exports = {run}