import {setupTests} from "./utils/utils";
import { getDefaultIdentities } from "./utils/identities";
import { Contract, ethers, getDefaultProvider, Wallet } from "ethers";
import CkErc20DepositAbi from "../out/ERC20DepositHelper.sol/CkErc20Deposit.json"
import { CkErc20Deposit } from "../types"

setupTests();

describe("E2E", () => {
    const { aliceIdentity, bobIdentity, charleIdentity } = getDefaultIdentities();
    const provider = getDefaultProvider("http://127.0.0.1:8545")

    const minterAddress = process.env.MINTER_ADDRESS;
    const ckErc20DepositAddress = process.env.CK_ERC20_DEPOSIT_ADDRESS
    const alicePrivateKey = process.env.ALICE_PRIVATE_KEY

    console.log("Minter adderess", process.env.MINTER_ADDRESS)
    console.log("CK_ERC20_DEPOSIT adderess", process.env.CK_ERC20_DEPOSIT_ADDRESS)

    const aliceSigner = new Wallet(alicePrivateKey, provider);
    
    const ckErc20DepositContract = new Contract(ckErc20DepositAddress, CkErc20DepositAbi.abi, aliceSigner) as unknown as CkErc20Deposit;

    it("allow user to bridge from EVM to ICP", async () => {
        const principalBytes = aliceIdentity.getPrincipal().toUint8Array(); 

        const result = await ckErc20DepositContract.deposit(minterAddress, ethers.parseUnits("1.0", 18), Buffer.alloc(32));
        
        console.log("result: ", result);
        // 30000000
    })
    
})