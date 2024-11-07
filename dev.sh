dfx killall
pkill anvil
pkill gnome-terminal

gnome-terminal \
    --tab --title="Anvil - EVM blockchain" -- bash -c "anvil --block-time 3; exec bash" 
gnome-terminal \
    --tab --title="DFX - IC blockchain" -- bash -c "dfx start --clean; exec bash"

sleep 10s
export ALICE_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
export BOB_PRIVATE_KEY=0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d

dfx deploy evm_rpc --argument '(record {})'

dfx canister call evm_rpc request '(variant {Custom=record {url="http://127.0.0.1:8545"}},"{\"jsonrpc\":\"2.0\",\"method\":\"eth_gasPrice\",\"params\":[],\"id\":1}",1000)' --wallet $(dfx identity get-wallet) --with-cycles 1000000000

dfx deploy eden_vault_backend --argument='(variant { InitArg = record {
    ethereum_network = variant { Local };
    ecdsa_key_name = "dfx_test_key";
    ethereum_contract_address = null : opt text;
    ledger_id = principal "apia6-jaaaa-aaaar-qabma-cai";
    ethereum_block_height = variant { Latest };
    minimum_withdrawal_amount = 10000000000000000000 : nat;
    next_transaction_nonce = 0 : nat;
    last_scraped_block_number = 0 : nat;
    admin = principal "nxu2q-em5br-fw5za-34owr-pxlfb-p6g73-6fe6i-sgggr-vtt27-hnxqk-gqe";
    ckerc20_token_address = "0x1234567890abcdef1234567890abcdef12345678" : text;
    ckerc20_token_symbol = "ckex" : text;
} })'


sleep 1s
MINTER_ADDRESS=$(dfx canister call eden_vault_backend minter_address --output json | sed 's/^.//;s/.$//')
export MINTER_ADDRESS

FORGE_TOKEN_DEPLOY_OUTPUT=$(forge create ./src/ForgeToken.sol:ForgeToken --rpc-url 127.0.0.1:8545 --private-key $ALICE_PRIVATE_KEY --constructor-args $MINTER_ADDRESS)
FORGE_TOKEN_ADDRESS=$(echo "$FORGE_TOKEN_DEPLOY_OUTPUT" | grep -oE "Deployed to: 0x[0-9a-fA-F]{40}" | sed 's/Deployed to: //')
export FORGE_TOKEN_ADDRESS


CK_ERC20_DEPOSIT_DEPLOY_OUTPUT=$(forge create ./src/ERC20DepositHelper.sol:CkErc20Deposit --rpc-url 127.0.0.1:8545 --private-key $ALICE_PRIVATE_KEY --constructor-args $MINTER_ADDRESS)
CK_ERC20_DEPOSIT_ADDRESS=$(echo "$CK_ERC20_DEPOSIT_DEPLOY_OUTPUT" | grep -oE "Deployed to: 0x[0-9a-fA-F]{40}" | sed 's/Deployed to: //')
export CK_ERC20_DEPOSIT_ADDRESS


echo "Upgrading canister"
dfx deploy eden_vault_backend --argument='(variant { UpgradeArg = record {
    next_transaction_nonce = opt 0 : opt nat;
    minimum_withdrawal_amount = opt 10_000_000_000 : opt nat;
    ethereum_contract_address = null : opt text;
    ethereum_block_height = opt variant { Latest } : opt variant { Latest; Safe; Finalized };
    erc20_helper_contract_address = opt "'"$CK_ERC20_DEPOSIT_ADDRESS"'" : opt text;
    last_erc20_scraped_block_number = null : opt nat;
    ledger_suite_orchestrator_id = null : opt principal;
    evm_rpc_id = opt principal "bkyz2-fmaaa-aaaaa-qaaaq-cai" : opt principal ;
    ckerc20_token_address = opt "'"$FORGE_TOKEN_ADDRESS"'" : opt text;
    ckerc20_token_symbol = opt "ckFT" : opt text;
} })' --upgrade-unchanged

cast send $MINTER_ADDRESS --value 100ether --rpc-url http://127.0.0.1:8545 --private-key $ALICE_PRIVATE_KEY

npx typechain --target ethers-v6 --out-dir ./types './out/**/ForgeToken.json'
npx typechain --target ethers-v6 --out-dir ./types './out/**/CkErc20Deposit.json'


