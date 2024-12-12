source ./dev_scripts/config.sh

cast send $EDEN_TOKEN_ADDRESS \
    --rpc-url $EVM_RPC_URL \
    --private-key $ALICE_PRIVATE_KEY \
    --gas-limit 65000 \
    "approve(address,uint256)" "$CK_ERC20_DEPOSIT_ADDRESS" "1000000000000"

# Deposit to:
#   principal: ogxac-f4uay-l5nc4-dth55-ubkk6-ulcjz-3i643-y2hwa-3cicp-gtbur-qae
#   byte principal: 1d940617d68b8399fbda054af51624e768f7378d1ec0d890279a61a460020000
cast send $CK_ERC20_DEPOSIT_ADDRESS \
    --rpc-url $EVM_RPC_URL \
    --private-key $ALICE_PRIVATE_KEY \
    --gas-limit 75000 \
    "deposit(address,uint256,bytes32)" "$EDEN_TOKEN_ADDRESS" "1000000000000" "1d940617d68b8399fbda054af51624e768f7378d1ec0d890279a61a460020000"

    # 1d2eb8183edacceb7f9d72c1b4cfa2a8d0261593e48a47f1a3eecf057c020000