source ./dev_scripts/config.sh

dfx killall
pkill anvil
pkill gnome-terminal
sleep 6s

gnome-terminal \
    --tab --title="Anvil - EVM blockchain" -- bash -c "anvil --block-time 3; exec bash" 
gnome-terminal \
    --tab --title="DFX - IC blockchain" -- bash -c "dfx start --clean; exec bash"
sleep 6s

dfx canister create eden_vault_backend
dfx canister create evm_rpc

forge build
FORGE_TOKEN_DEPLOY_OUTPUT=$(forge create ./src/ForgeToken.sol:ForgeToken --rpc-url $EVM_RPC_URL --private-key $ALICE_PRIVATE_KEY)
FORGE_TOKEN_ADDRESS=$(echo "$FORGE_TOKEN_DEPLOY_OUTPUT" | grep -oE "Deployed to: 0x[0-9a-fA-F]{40}" | sed 's/Deployed to: //')
export FORGE_TOKEN_ADDRESS

EDEN_TOKEN_DEPLOY_OUTPUT=$(forge create ./src/EdenTokenLL.sol:EdenTokenLL --rpc-url $EVM_RPC_URL --private-key $ALICE_PRIVATE_KEY --constructor-args 10000000000000000000000000000 "tEDEN" "tEDN" "$ALICE_PUB_KEY" "$ALICE_PUB_KEY" 0 "0x0000000000000000000000000000000000000000")
EDEN_TOKEN_ADDRESS=$(echo "$EDEN_TOKEN_DEPLOY_OUTPUT" | grep -oE "Deployed to: 0x[0-9a-fA-F]{40}" | sed 's/Deployed to: //')
export EDEN_TOKEN_ADDRESS

cast send $EDEN_TOKEN_ADDRESS \
    --rpc-url $EVM_RPC_URL \
    --private-key $ALICE_PRIVATE_KEY \
    --gas-limit 65000 \
    "proposeLosslessTurnOff()"

cast send $EDEN_TOKEN_ADDRESS \
    --rpc-url $EVM_RPC_URL \
    --private-key $ALICE_PRIVATE_KEY \
    --gas-limit 65000 \
    "executeLosslessTurnOff()"

dfx deploy evm_rpc --argument '(record {})'
set -o allexport; source .env; set +o allexport

dfx canister call evm_rpc request "(variant {Custom=record {url=\"http://$EVM_RPC_URL\"}},"{\"jsonrpc\":\"2.0\",\"method\":\"eth_gasPrice\",\"params\":[],\"id\":1}",1000)" --wallet $(dfx identity get-wallet) --with-cycles 1000000000

dfx deploy eden_vault_backend --argument='(variant { InitArg = record {
    ethereum_network = variant { Local };
    ecdsa_key_name = "dfx_test_key";
    ethereum_block_height = variant { Latest };
    minimum_withdrawal_amount = 5_000_000_000_000;
    next_transaction_nonce = 0;
    last_scraped_block_number = 0;
    admin = principal "nxu2q-em5br-fw5za-34owr-pxlfb-p6g73-6fe6i-sgggr-vtt27-hnxqk-gqe";
    ckerc20_token_address = "'"$EDEN_TOKEN_ADDRESS"'";
    ckerc20_token_symbol = "ckEDEN";
} })'

MINTER_ADDRESS=$(dfx canister call eden_vault_backend minter_address --output json | sed 's/^.//;s/.$//')
export MINTER_ADDRESS

sleep 1s

CK_ERC20_DEPOSIT_DEPLOY_OUTPUT=$(forge create ./src/ERC20DepositHelper.sol:CkErc20Deposit --rpc-url $EVM_RPC_URL --private-key $ALICE_PRIVATE_KEY --constructor-args $MINTER_ADDRESS)
CK_ERC20_DEPOSIT_ADDRESS=$(echo "$CK_ERC20_DEPOSIT_DEPLOY_OUTPUT" | grep -oE "Deployed to: 0x[0-9a-fA-F]{40}" | sed 's/Deployed to: //')
export CK_ERC20_DEPOSIT_ADDRESS


echo "Upgrading canister"
dfx deploy eden_vault_backend --argument='(variant { UpgradeArg = record {
    next_transaction_nonce = opt 0;
    minimum_withdrawal_amount = opt 10_000_000_000;
    ethereum_block_height = opt variant { Latest };
    erc20_helper_contract_address = opt "'"$CK_ERC20_DEPOSIT_ADDRESS"'";
    last_erc20_scraped_block_number = null;
    evm_rpc_id = opt principal "'"$CANISTER_ID_EVM_RPC"'";
    withdraw_fee_value = opt 4_000_000;
} })' --upgrade-unchanged

cast send $MINTER_ADDRESS --value 100ether --rpc-url http://$EVM_RPC_URL --private-key $ALICE_PRIVATE_KEY

npx typechain --target ethers-v6 --out-dir ./types './out/**/ForgeToken.json'
npx typechain --target ethers-v6 --out-dir ./types './out/**/CkErc20Deposit.json'

cast send $TEST_WALLET --value 100ether --rpc-url http://$EVM_RPC_URL --private-key $ALICE_PRIVATE_KEY
cast send $FORGE_TOKEN_ADDRESS --rpc-url http://$EVM_RPC_URL --private-key $ALICE_PRIVATE_KEY --gas-limit 65000 "transfer(address,uint256)" "$TEST_WALLET" "10000000000000000"
cast send $EDEN_TOKEN_ADDRESS --rpc-url http://$EVM_RPC_URL --private-key $ALICE_PRIVATE_KEY --gas-limit 65000 "transfer(address,uint256)" "$TEST_WALLET" "10000000000000000"

echo "-------------------------------------------------------"
echo "Environment variables:"
echo "-------------------------------------------------------"
echo "MINTER_ADDRESS=$MINTER_ADDRESS"
echo "FORGE_TOKEN_ADDRESS=$FORGE_TOKEN_ADDRESS"
echo "CK_ERC20_DEPOSIT_ADDRESS=$CK_ERC20_DEPOSIT_ADDRESS"
echo "EDEN_TOKEN_ADDRESS=$EDEN_TOKEN_ADDRESS"
echo "-------------------------------------------------------"