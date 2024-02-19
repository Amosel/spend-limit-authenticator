/**
* This file was automatically generated by @cosmwasm/ts-codegen@0.35.7.
* DO NOT MODIFY IT BY HAND. Instead, modify the source JSONSchema file,
* and run the @cosmwasm/ts-codegen generate command to regenerate this file.
*/
import { CosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { SpendingResponse, SpendingsByAccountResponse } from "./SpendLimit.types";
export interface SpendLimitReadOnlyInterface {
    contractAddress: string;
    spending: ({ account, subkey }: {
        account: string;
        subkey: string;
    }) => Promise<SpendingResponse>;
    spendingsByAccount: ({ account }: {
        account: string;
    }) => Promise<SpendingsByAccountResponse>;
}
export declare class SpendLimitQueryClient implements SpendLimitReadOnlyInterface {
    client: CosmWasmClient;
    contractAddress: string;
    constructor(client: CosmWasmClient, contractAddress: string);
    spending: ({ account, subkey }: {
        account: string;
        subkey: string;
    }) => Promise<SpendingResponse>;
    spendingsByAccount: ({ account }: {
        account: string;
    }) => Promise<SpendingsByAccountResponse>;
}
//# sourceMappingURL=SpendLimit.client.d.ts.map