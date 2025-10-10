#!/bin/bash
function open_hrmp_channels() {
    local relay_url=$1
    local relay_chain_seed=$2
    local sender_para_id=$3
    local recipient_para_id=$4
    local max_capacity=$5
    local max_message_size=$6
    echo "  calling open_hrmp_channels:"
    echo "      relay_url: ${relay_url}"
    echo "      relay_chain_seed: ${relay_chain_seed}"
    echo "      sender_para_id: ${sender_para_id}"
    echo "      recipient_para_id: ${recipient_para_id}"
    echo "      max_capacity: ${max_capacity}"
    echo "      max_message_size: ${max_message_size}"
    echo "      params:"
    echo "--------------------------------------------------"

    polkadot-js-api \
        --ws "${relay_url?}" \
        --seed "${relay_chain_seed?}" \
        --sudo \
        tx.hrmp.forceOpenHrmpChannel \
            ${sender_para_id} \
            ${recipient_para_id} \
            ${max_capacity} \
            ${max_message_size}
}

open_hrmp_channels \
    "ws://127.0.0.1:8833" \
    "//Alice" \
    4502 \
    4594 \
    8 \
    1024
open_hrmp_channels \
    "ws://127.0.0.1:8833" \
    "//Alice" \
    4594 \
    4502 \
    8 \
    1024