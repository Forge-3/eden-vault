# This script starts the development environment for eden_vault

gnome-terminal \
    --tab --title="Anvil - EVM blockchain" -- bash -c "anvil; exec bash" 
gnome-terminal \
    --tab --title="DFX - IC blockchain" -- bash -c "dfx start --clean; exec bash"

sleep 15s

dfx deploy eden_vault_backend --argument='(variant { InitArg = record {
    ethereum_network = variant { Sepolia };
    ecdsa_key_name = "test_key_1";
    ethereum_contract_address = null;
    ledger_id = principal "apia6-jaaaa-aaaar-qabma-cai";
    ethereum_block_height = variant { Latest };
    minimum_withdrawal_amount = 10000000000000000000 : nat;
    next_transaction_nonce = 0 : nat;
    last_scraped_block_number = 0 : nat;
} })'

MINTER_ADDRESS=0x7574eB42cA208A4f6960ECCAfDF186D627dCC175
forge create ./src/ERC20DepositHelper.sol:CkErc20Deposit --rpc-url 127.0.0.1:8545 --account alice --constructor-args $MINTER_ADDRESS