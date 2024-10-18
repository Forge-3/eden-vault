# This script starts the development environment for eden_vault
dfx killall
pkill anvil
pkill gnome-terminal

gnome-terminal \
    --tab --title="Anvil - EVM blockchain" -- bash -c "anvil; exec bash" 
gnome-terminal \
    --tab --title="DFX - IC blockchain" -- bash -c "dfx start --clean; exec bash"

sleep 10s

dfx deploy eden_vault_backend --argument='(variant { InitArg = record {
    ethereum_network = variant { Local };
    ecdsa_key_name = "dfx_test_key";
    ethereum_contract_address = null;
    ledger_id = principal "apia6-jaaaa-aaaar-qabma-cai";
    ethereum_block_height = variant { Latest };
    minimum_withdrawal_amount = 10000000000000000000 : nat;
    next_transaction_nonce = 0 : nat;
    last_scraped_block_number = 0 : nat;
} })'

sleep 1s

export ALICE_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
export BOB_PRIVATE_KEY=0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d

MINTER_ADDRESS=$(dfx canister call eden_vault_backend minter_address --output json | sed 's/^.//;s/.$//')
export MINTER_ADDRESS

CK_ERC20_DEPOSIT_DEPLOY_OUTPUT=$(forge create ./src/ERC20DepositHelper.sol:CkErc20Deposit --rpc-url 127.0.0.1:8545 --private-key $ALICE_PRIVATE_KEY --constructor-args $MINTER_ADDRESS)
CK_ERC20_DEPOSIT_ADDRESS=$(echo "$CK_ERC20_DEPOSIT_DEPLOY_OUTPUT" | grep -oE "Deployed to: 0x[0-9a-fA-F]{40}" | sed 's/Deployed to: //')
export CK_ERC20_DEPOSIT_ADDRESS

FORGE_TOKEN_DEPLOY_OUTPUT=$(forge create ./src/ForgeToken.sol:ForgeToken --rpc-url 127.0.0.1:8545 --private-key $ALICE_PRIVATE_KEY --constructor-args $MINTER_ADDRESS)
FORGE_TOKEN_ADDRESS=$(echo "$FORGE_TOKEN_DEPLOY_OUTPUT" | grep -oE "Deployed to: 0x[0-9a-fA-F]{40}" | sed 's/Deployed to: //')
export FORGE_TOKEN_ADDRESS

npx typechain --target ethers-v6 --out-dir ./types './out/**/ForgeToken.json'
npx typechain --target ethers-v6 --out-dir ./types './out/**/CkErc20Deposit.json'