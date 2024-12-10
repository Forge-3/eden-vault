import {setupTests} from "./utils/utils";
import { getDefaultIdentities } from "./utils/identities";
import { AbstractProvider, BytesLike, Contract, decodeBase64, ethers, getDefaultProvider, Wallet } from "ethers";
import CkErc20DepositAbi from "../out/ERC20DepositHelper.sol/CkErc20Deposit.json"
import ForgeTokenAbi from "../out/ForgeToken.sol/ForgeToken.json"
import { CkErc20Deposit } from "../types"
import {ForgeToken} from "../types/ForgeToken"
import {expect} from "chai";
import { Actor, HttpAgent } from "@dfinity/agent";
import { idlFactory } from "../src/declarations/eden_vault_backend";
import { canisterId } from "../src/declarations/eden_vault_backend"
import { Principal } from "@dfinity/principal";

const getNonce = async (provider: AbstractProvider, signer: Wallet) =>{
    return await provider.getTransactionCount(signer.address, "latest") + 1
}

setupTests();

describe("E2E", () => {
    const {aliceIdentity, bobIdentity, charleIdentity} = getDefaultIdentities();
    const provider = getDefaultProvider("http://127.0.0.1:8545")

    const alicePrivateKey = process.env.ALICE_PRIVATE_KEY

    const minterAddress = process.env.MINTER_ADDRESS;
    const ckErc20DepositAddress = process.env.CK_ERC20_DEPOSIT_ADDRESS
    const forgeTokenAddress = process.env.EDEN_TOKEN_ADDRESS

    console.log("MINTER_ADDRESS adderess", minterAddress)
    console.log("CK_ERC20_DEPOSIT adderess", ckErc20DepositAddress)
    console.log("FORGE_TOKEN adderess", forgeTokenAddress)

    console.log("Alice principal: ", aliceIdentity.getPrincipal().toString())

    const aliceSigner = new Wallet(alicePrivateKey, provider);

    const ckErc20DepositContract = new Contract(ckErc20DepositAddress, CkErc20DepositAbi.abi, aliceSigner) as unknown as CkErc20Deposit;
    const forgeTokenContract = new Contract(forgeTokenAddress, ForgeTokenAbi.abi, aliceSigner) as unknown as ForgeToken;

    const aliceMinterAgent = HttpAgent.createSync({
        host: "http://127.0.0.1:4943",
        identity: aliceIdentity,
    });
    aliceMinterAgent.fetchRootKey();

    const aliceMinterActor = Actor.createActor(idlFactory, {
        agent: aliceMinterAgent,
        canisterId: canisterId,
    });
    
    it("should allow admin to set a new admin", async () => {
        const newAdminPrincipal = bobIdentity.getPrincipal();
        
        const result = await aliceMinterActor.set_admin(newAdminPrincipal);
        expect(result).to.deep.equal({ Ok: null });

        const result2 = await aliceMinterActor.set_admin(aliceIdentity.getPrincipal());
        expect(result2).to.deep.equal({ Err: 'Only the current admin can set a new admin.' });
    });

    it("allow user to bridge from EVM to ICP", async () => {
        const amount = ethers.parseUnits("1.0", 9);
        const approveResult = await forgeTokenContract.approve(ckErc20DepositAddress, amount, {
            nonce: await getNonce(provider, aliceSigner)
        });
        //  console.log("approveResult: ", approveResult);

        const depositPrincipal = aliceIdentity.getPrincipal();
        console.log("depositPrincipal: ", depositPrincipal);
        console.log("depositPrincipal: ", depositPrincipal.toString());
        const depositPrincipalBytes = depositPrincipal.toUint8Array();

        await ckErc20DepositContract.once(ckErc20DepositContract.filters.ReceivedErc20, () => {
            console.log("emitted event")
        })
        const paddedPrincipalBytes = Buffer.alloc(32);
        paddedPrincipalBytes.set(depositPrincipalBytes, 0);
        console.log("paddedPrincipalBytes: ", paddedPrincipalBytes);

        console.log("Alice balance: ", await aliceMinterActor.erc20_balance_of(depositPrincipal));

        const depositResult = await ckErc20DepositContract.deposit(forgeTokenAddress, amount, paddedPrincipalBytes, {
                gasLimit: 1000000,
                nonce: await getNonce(provider, aliceSigner)
        });
        // console.log("depositResult: ", depositResult);

        console.log("Alice balance: ", await aliceMinterActor.erc20_balance_of(depositPrincipal));
        await new Promise(resolve => setTimeout(resolve, 60000));
        console.log("Alice balance: ", await aliceMinterActor.erc20_balance_of(depositPrincipal));
        
    })

})

