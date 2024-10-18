import {should, use} from "chai";
import chaiAsPromised from "chai-as-promised";

export const setupTests = () => {
    should();
    use(chaiAsPromised);

    (BigInt.prototype as any).toJSON = function (): number {
        return this.toString();
    };
}

export const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export const host =
    process.env.DFX_NETWORK === "local" ? "http://localhost:4943" : undefined;

export function base64ToBytes(base64: string) {
    const binString = atob(base64);
    return Uint8Array.from(binString, (m) => m.codePointAt(0));
}
  
export function bytesToBase64(bytes: Uint8Array) {
    const binString = Array.from(bytes, (byte) =>
      String.fromCodePoint(byte),
    ).join("");
    return btoa(binString);
}