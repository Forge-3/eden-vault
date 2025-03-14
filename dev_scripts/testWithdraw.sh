source ./dev_scripts/config.sh

dfx canister call eden_vault_backend withdraw_erc20 '(record {
	recipient = "'"$ALICE_PUB_KEY"'";
	amount = 100000000000
})'

# cast call $EDEN_TOKEN_ADDRESS \
#     --rpc-url $EVM_RPC_URL \
#     --gas-limit 65000 \
#     "balanceOf(address)" "$BOB_PUB_KEY"