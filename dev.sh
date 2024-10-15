# This script starts the development environment for eden_vault

gnome-terminal \
    --tab --title="Anvil - EVM blockchain" -- bash -c "anvil; exec bash" 
gnome-terminal \
    --tab --title="DFX - IC blockchain" -- bash -c "dfx start; exec bash"
