import { Ed25519KeyIdentity } from "@dfinity/identity"
import { createHash } from "crypto"

export const getDefaultIdentities = () => {
    const alice = new Uint8Array(createHash('sha256').update('alice').digest())
    const bob = new Uint8Array(createHash('sha256').update('bob').digest())
    const charle = new Uint8Array(createHash('sha256').update('charle').digest())

    const aliceIdentity = Ed25519KeyIdentity.generate(alice)
    const bobIdentity = Ed25519KeyIdentity.generate(bob)
    const charleIdentity = Ed25519KeyIdentity.generate(charle)

    return { aliceIdentity, bobIdentity, charleIdentity }
}