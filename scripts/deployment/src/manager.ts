import { ExecuteResult, SigningCosmWasmClient } from "@cosmjs/cosmwasm-stargate";
import { StdFee } from "@cosmjs/stargate";
import * as fs from "fs"
import { config } from "dotenv"
config({ path: '.env' })
const denom: string = process.env.DENOM

export class ManagerClient {
	client: SigningCosmWasmClient;

	constructor(client: SigningCosmWasmClient) {
		this.client = client;
	}

	async deploy(artifactsRoot: string, sender: string, factoryAddress: string, uploadGas: StdFee, executeGas: StdFee): Promise<[number, string]> {
		const wasm = fs.readFileSync(`${artifactsRoot}/croncat_manager.wasm`)
		const uploadRes = await this.client.upload(sender, wasm, uploadGas)
		const codeId = uploadRes.codeId

		let base64ManagerInst = Buffer.from(JSON.stringify({
			"denom": denom,
			"croncat_factory_addr": factoryAddress,
			"croncat_tasks_key": ["tasks", [0, 0]],
			"croncat_agents_key": ["agents", [0, 0]]
		})).toString('base64')

		// instantiate manager contract (from the factory)
		const deployMsg = {
			"deploy": {
				"kind": "manager",
				"module_instantiate_info": {
					"code_id": codeId,
					"version": [0, 1],
					"commit_id": "6ffbf4aa3617f978a07b594adf8013f19a936331",
					"checksum": "df0d27c5de3011a2e3f9789b1ac9c2b984923762dfcce45f1cda4f42a12e0525",
					"changelog_url": "https://github.com/croncats",
					"schema": "",
					"msg": base64ManagerInst,
					"contract_name": "manager"
				}
			}
		}

		const instRes = await this.client.execute(sender, factoryAddress, deployMsg, executeGas);
		const address: string = instRes.logs[0].events[1].attributes[0].value

		return [codeId, address];
	}
}