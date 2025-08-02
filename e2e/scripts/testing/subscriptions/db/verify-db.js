async function run(nodeName, networkInfo, args) {
    const {wsUri, userDefinedTypes} = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);

    await zombie.util.cryptoWaitReady();

    // Credits and Frequency pulled from sudoQuoteSubscription using credits = 100, frequency = 100
    const subId = args[0];
    const subscriber = args[1];
    const state = args[2];
    const credits = Number(args[3]);    
    const frequency = Number(args[4]);

    const subInfo = await api.query.idnManager.subscriptions(subId);

    const subInfoJson = JSON.parse(subInfo.toString());
    
    if (!(subscriber === subInfoJson.details.subscriber)) {
        throw new Error(`Subscriber info mismatch expected ${subscriber} but got ${subInfoJson.details.subscriber}`)
    }
    if (!(state === subInfoJson.state)) {
        throw new Error(`State info mismatch expected ${state} but got ${subInfoJson.state}`)
    }
    if (!(credits === subInfoJson.credits)) {
        throw new Error(`Credits mismatch expected ${credits} but got ${subInfoJson.credits}`)
    }
    if (!(frequency === subInfoJson.frequency)) {
        throw new Error(`Frequency mismatch expected ${frequency} but got ${subInfoJson.frequency}`)
    }
}

module.exports = {run}