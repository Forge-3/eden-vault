# `eden_vault`

This repository contains a treasury developed based on the functionality of ckERC20 within the ICP blockchain ecosystem. This technology utilizes HTTP outcalls to query multiple JSON-RPC providers to interact with the BSC Testnet blockchain.


<b>BSC Testnet addresses:</b>

|  Name     |  Address                                    |
| --------  | ------------------------------------------  |
| Minter    | 0x7187CC02Be5f744eE15653A4ea3F13FeC23E1a7B  |
| Token     | 0x890e77bA80f1D3f62Fe5d9259750f0C52F198b0f  |
| Deposit   | 0xaDc4434fD98D5dA23Bc0DCd6C281BBE2E021D7c1  |

<b>ICP addresses:</b>

|  Name     |  Principal                                  |
| --------  | ------------------------------------------  |
| Minter    | be2us-64aaa-aaaaa-qaabq-cai                 |

## Architecture

The Eden Treasury is responsible for depositing and withdrawing the ERC-20 token. Each deposit involves invoking the `deposit` method on the smart contract (CkErc20Deposit) on the BSC Testnet network.

## Description of deposit

1. Depositing tokens into the smart contract triggers the emission of the "ReceivedErc20" event, which contains all the information related to the deposit.  
2. The Eden Treasury periodically reads the latest events from the CkErc20Deposit contract and subsequently credits the transferred tokens to the recipient's account.

Flow:

----
<pre>
 ┌────┐                      ┌───────────────┐                     ┌───────────────┐                        ┌──────┐
 │User│                      │ERC-20 Contract│                     │Helper Contract│                        │Vault │
 └─┬──┘                      └───────┬───────┘                     └───────┬───────┘                        └──┬───┘
   │                                 │                                     │                                   │
   │approve(helper_contract, amount) │                                     │                                   │
   │────────────────────────────────>│                                     │                                   │
   │                                 │                                     │                                   │
   │                deposit(amount, principal)                             │                                   │
   │──────────────────────────────────────────────────────────────────────>│                                   │
   │                                 │ transferFrom(user, minter, amount)  │                                   │
   │                                 │<────────────────────────────────────│                                   │
   │                                 │                                     │                                   │
   │                                 │                                     │       get_events                  │
   │                                 │                                     │<──────────────────────────────────│
   │                                 │                                     │Events(token_id, amount, principal)│
   │                                 │                                     │──────────────────────────────────>│
   │                                 │  asign_tokens(amount, principal)    │                                   │
   │<──────────────────────────────────────────────────────────────────────────────────────────────────────────│
 ┌─┴──┐                      ┌───────┴───────┐                     ┌───────┴───────┐                        ┌──┴───┐
 │User│                      │ERC-20 Contract│                     │Helper Contract│                        │Vault |
 └────┘                      └───────────────┘                     └───────────────┘                        └──────┘
 </pre>
----

## Description of withdraw

1. Calling the "withdraw_erc20" method results in the tokens being deducted from the treasury and a transaction being created and sent. If the transaction fails, it is resent.

----
<pre>
 ┌────┐                                                 ┌──────┐                              ┌───────────────────┐
 │User│                                                 │Vault │                              │BSC Testnet Network│
 └─┬──┘                                                 └──┬───┘                              └─────┬─────────────┘
   │                                                       │                                        │
   │    withdraw_erc20(amount, destination_eth_address)    │                                        │
   │──────────────────────────────────────────────────────>│                                        │
   │                                                       │ eth_sendRawTransaction                 │
   │                                                       │ (destination_eth_address, amount)      │
   │                                                       │───────────────────────────────────────>│
 ┌─┴──┐                                                 ┌──┴───┐                              ┌─────┴─────────────┐
 │User│                                                 │Vault │                              │BSC Testnet Network│
 └────┘                                                 └──────┘                              └───────────────────┘
 </pre>
----

## How to start

If you want to start working on your project right away, you might want to try the following commands:

```bash
cd eden_vault/
dfx help
dfx canister --help
```

## Running the project locally

If you want to test your project locally, you can use the following commands:

```bash
# Starts the replica, running in the background
dfx start --background

# Deploys your canisters to the replica and generates your candid interface
dfx deploy
```

Once the job completes, your application will be available at `http://localhost:4943?canisterId={asset_canister_id}`.

If you have made changes to your backend canister, you can generate a new candid interface with

```bash
npm run generate
```

at any time. This is recommended before starting the frontend development server, and will be run automatically any time you run `dfx deploy`.

If you are making frontend changes, you can start a development server with

```bash
npm start
```

Which will start a server at `http://localhost:8080`, proxying API requests to the replica at port 4943.

