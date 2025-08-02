async function run(nodeName, networkInfo, args) {
    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    await zombie.util.cryptoWaitReady();

    // Credits and Frequency pulled from sudoQuoteSubscription using credits = 100, frequency = 100
    const subId = args[0];

    const subInfo = await api.query.idnManager.subscriptions(subId);

    const subInfoString = subInfo.toString();

    if(subInfoString) {
        console.log(subInfoString)
        throw new Error("Subscription still in DB after subscription deletion");
    }
}

module.exports = {run}