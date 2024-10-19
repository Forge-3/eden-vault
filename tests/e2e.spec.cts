import {setupTests} from "./utils/utils";
import { getDefaultIdentities } from "./utils/identities";
import { Contract, ethers, getDefaultProvider, Wallet } from "ethers";
import CkErc20DepositAbi from "../out/ERC20DepositHelper.sol/CkErc20Deposit.json"
import ForgeTokenAbi from "../out/ForgeToken.sol/ForgeToken.json"
import { CkErc20Deposit } from "../types"
import {ForgeToken, ForgeTokenInterface} from "../types/ForgeToken"
import {expect} from "chai";

setupTests();

describe("E2E",  () => {
    const {aliceIdentity, bobIdentity, charleIdentity} = getDefaultIdentities();
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
    const forgeTokenContract = new Contract(forgeTokenAddress, ForgeTokenAbi.abi, aliceSigner) as unknown as ForgeToken;

    it("allow user to bridge from EVM to ICP", async () => {

        const amount = ethers.parseUnits("1.0", 9);
        const approveResult = await forgeTokenContract.approve(ckErc20DepositAddress, amount, {
            nonce: await provider.getTransactionCount(aliceSigner.address, "latest")
        });
        console.log("approveResult: ", approveResult)

        await ckErc20DepositContract.once(ckErc20DepositContract.filters.ReceivedErc20, () => {
            console.log("emitted event")
        })

        const depositResult =
            await ckErc20DepositContract.deposit(forgeTokenAddress, amount, Buffer.alloc(32), {
                gasLimit: 1000000,
                nonce: await provider.getTransactionCount(aliceSigner.address, "latest")
            });

        console.log("depositResult: ", depositResult)
    })

})