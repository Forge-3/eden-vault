import {setupTests} from "./utils/utils";
import { getDefaultIdentities } from "./utils/identities";
import { Contract, ethers, getDefaultProvider, Wallet } from "ethers";
import CkErc20DepositAbi from "../out/ERC20DepositHelper.sol/CkErc20Deposit.json"
import ForgeTokenAbi from "../out/ForgeToken.sol/ForgeToken.json"
import { CkErc20Deposit } from "../types"
import { ForgeTokenInterface } from "../types/ForgeToken"

setupTests();

describe("E2E", () => {
    const { aliceIdentity, bobIdentity, charleIdentity } = getDefaultIdentities();
    const provider = getDefaultProvider("http://127.0.0.1:8545")

    const alicePrivateKey = process.env.ALICE_PRIVATE_KEY

    const minterAddress = process.env.MINTER_ADDRESS;
    const ckErc20DepositAddress = process.env.CK_ERC20_DEPOSIT_ADDRESS
    const forgeTokenAddress = process.env.FORGE_TOKEN_ADDRESS;

    console.log("MINTER_ADDRESS adderess", minterAddress)
    console.log("CK_ERC20_DEPOSIT adderess", ckErc20DepositAddress)
    console.log("FORGE_TOKEN adderess", forgeTokenAddress)

    const aliceSigner = new Wallet(alicePrivateKey, provider);
    
    const ckErc20DepositContract = new Contract(ckErc20DepositAddress, CkErc20DepositAbi.abi, aliceSigner) as unknown as CkErc20Deposit;
    const forgeTokenContract = new Contract(forgeTokenAddress, ForgeTokenAbi.abi, aliceSigner) as unknown as ForgeTokenInterface;

    it("allow user to bridge from EVM to ICP", async () => {
        const principalBytes = aliceIdentity.getPrincipal().toUint8Array();

        const result =
            await ckErc20DepositContract.deposit(forgeTokenAddress, ethers.parseUnits("1.0", 9), Buffer.alloc(32), {
            gasLimit: 1000000
        });
        console.log("result: ", result);

    })

})