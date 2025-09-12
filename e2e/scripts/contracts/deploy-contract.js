const path = require('path');
const fs = require('fs');

async function run(nodeName, networkInfo, args) {
    const { wsUri, userDefinedTypes } = networkInfo.nodesByName[nodeName];
    const api = await zombie.connect(wsUri, userDefinedTypes);
    await zombie.util.cryptoWaitReady();

    // Read the contract JSON file.
    const contractJsonPath = path.resolve(__dirname, '../../resources/example.contract');
    const contractJson = JSON.parse(fs.readFileSync(contractJsonPath, 'utf8'));

    // Prepare the transaction details.
    const keyring = new zombie.Keyring({ type: 'sr25519' });
    const alice = keyring.addFromUri('//Alice');

    // Get the constructor metadata from the contract's ABI.
    const constructorAbi = contractJson.spec.constructors[1];
    if (!constructorAbi) {
        throw new Error('Constructor not found in contract metadata.');
    }

    // console.log('constructorAbi')
    // console.log(constructorAbi)

    const selector = api.registry.createType('Bytes', constructorAbi.selector).toU8a();
    
    // Create the constructor data as hex string
    const data = '0x' + Buffer.from(selector).toString('hex');

    // Get the contract Wasm code as hex string
    const wasmBuffer = Buffer.from(contractJson.source.wasm, 'base64');
    const wasm = '0x' + wasmBuffer.toString('hex');

    const BN = api.registry.createType('u64', 0).constructor;    
    const gasLimit = api.registry.createType('WeightV2', {
        refTime: new BN('100000000000'),  // 100 billion
        proofSize: new BN('100000'),      // 100 thousand
    });
    
    const storageDepositLimit = null;
    const value = api.createType('Balance', 0);

    // Send the transaction using polkadotjs
    const salt = '0x';
    console.log('sending contract initiate with code tx')
    const unsub = await api.tx.contracts
        .instantiateWithCode(value, gasLimit, storageDepositLimit, wasm, data, salt)
        .signAndSend(alice, ({ status, events }) => {
            console.log('sent, waiting to be included in a block')
            if (status.isInBlock || status.isFinalized) {
                const instantiateEvent = events.find(({ event }) =>
                    api.events.contracts.Instantiated.is(event)
                );

                if (instantiateEvent) {
                    const [, contractAddress] = instantiateEvent.event.data;
                    console.log(`Contract deployed at: ${contractAddress.toString()}`);
                    console.log('Contract deployment successful!');
                    process.exit(0);
                } else {
                    console.error('Contract deployment failed: Instantiation event not found.');
                    console.log('Events:', events.map(e => e.event.method));
                    process.exit(1);
                }
                unsub();
            }
        });
}

module.exports = { run };