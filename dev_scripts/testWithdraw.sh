source ./dev_scripts/config.sh


# Set transfer limitation
#
# cast send $EDEN_TOKEN_ADDRESS \
#     --rpc-url $EVM_RPC_URL \
#     --private-key $ALICE_PRIVATE_KEY \
#     --gas-limit 75000 \
#     "setRestrictionActive(bool)" "true"

# cast send $EDEN_TOKEN_ADDRESS \
#     --rpc-url $EVM_RPC_URL \
#     --private-key $ALICE_PRIVATE_KEY \
#     --gas-limit 75000 \
#     "setMaxTransferAmount(uint256)" "10000000"

dfx canister call eden_vault_backend withdraw_erc20 '(record {
	recipient = "'"$ALICE_PUB_KEY"'";
	amount = 100000000000
})'

# cast call $EDEN_TOKEN_ADDRESS \
#     --rpc-url $EVM_RPC_URL \
#     --gas-limit 65000 \
#     "balanceOf(address)" "$BOB_PUB_KEY"