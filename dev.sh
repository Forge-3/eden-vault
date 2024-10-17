# This script starts the development environment for eden_vault

gnome-terminal \
    --tab --title="Anvil - EVM blockchain" -- bash -c "anvil; exec bash" 
gnome-terminal \
    --tab --title="DFX - IC blockchain" -- bash -c "dfx start --clean; exec bash"

sleep 15s

dfx deploy

MINTER_ADDRESS=0x7574eB42cA208A4f6960ECCAfDF186D627dCC175
forge create ./src/ERC20DepositHelper.sol:CkErc20Deposit --rpc-url 127.0.0.1:8545 --account alice --constructor-args $MINTER_ADDRESS