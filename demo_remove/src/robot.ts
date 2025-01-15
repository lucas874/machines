import { createMachineRunner } from '@actyx/machine-runner'
import { Actyx } from '@actyx/sdk'
import { Events, manifest, protocol } from './protocol'

const machine = protocol.makeMachine('robot')
export const Idle = machine.designEmpty('Idle').finish()
export const WateringPlant = machine.designEmpty('WateringPlant').finish()
Idle.react([Events.NeedsWater], WateringPlant, (_) => WateringPlant.make())
WateringPlant.react([Events.HasWater], Idle, (_) => Idle.make())
var m = machine.createJSONForAnalysis(Idle)
const [m2, i2] = protocol.makeProjMachine("robot", m, Events.All)

export async function main() {
  const sdk = await Actyx.of(manifest)
  const tags = protocol.tagWithEntityId('robot-1')
  const machine = createMachineRunner(sdk, tags, i2, undefined)

  for await (const state of machine) {
    console.log(state)
  }
}

main()