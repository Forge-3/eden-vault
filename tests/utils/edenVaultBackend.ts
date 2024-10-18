import {Identity} from "@dfinity/agent";
import {createActor, canisterId} from "../../src/declarations/eden_vault_backend";
import {host} from "./utils";

export const getLlmOnchainBackend = (identity?: Identity) => {
    return createActor(canisterId, {
        agentOptions: { host, identity },
    });
};