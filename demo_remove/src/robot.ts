import { createMachineRunner } from '@actyx/machine-runner'
import { Actyx } from '@actyx/sdk'
import { Events, manifest, protocol } from './protocol'
type SpentWater = {
  /// Tracks the latest amount of water used
  lastMl: number
  /// Tracks the total amount of water used
  totalMl: number
}
const machine = protocol.makeMachine('robot')
//export const Idle = machine.designEmpty('Idle').finish()
//export const WateringPlant = machine.designEmpty('WateringPlant').finish()
export const Idle = machine.designState('Idle').withPayload<SpentWater>().finish()
export const WateringPlant = machine.designState('WateringPlant').withPayload<SpentWater>().finish()

//Idle.react([Events.NeedsWater], WateringPlant, (_) => WateringPlant.make())
//WateringPlant.react([Events.HasWater], Idle, (_) => Idle.make())
Idle.react([Events.NeedsWater], WateringPlant, (state, event) => {
  console.log(`The plant is requesting ${event.payload.requiredWaterMl} ml of water!`)
  const newStatePayload = {
    lastMl: event.payload.requiredWaterMl,
    totalMl: state.self.totalMl + event.payload.requiredWaterMl,
  }
  console.log(`Total water consumption: ${newStatePayload.totalMl}`)
  return WateringPlant.make(newStatePayload)
})

WateringPlant.react([Events.HasWater], Idle, (state, _) => Idle.make(state.self))

var m = machine.createJSONForAnalysis(Idle)
//const [m2, i2] = protocol.makeProjMachine("robot", m, Events.All)
const cMap = new Map()
const rMap = new Map()
const statePayloadMap = new Map()
const fMap : any = {commands: cMap, reactions: rMap, statePayloads: statePayloadMap}
//const [m3, i3] = protocol.extendMachine("robot", m, Events.All, [machine, Idle], fMap)
const [m3, i3] = protocol.extendMachine("robot", m, Events.All, fMap)
export async function main() {
  const sdk = await Actyx.of(manifest)
  const tags = protocol.tagWithEntityId('robot-1')

  //const machine = createMachineRunner(sdk, tags, i2, undefined)
  const machine = createMachineRunner(sdk, tags, Idle, {
    lastMl: 0,
    totalMl: 0,
  })
  for await (const state of machine) {
    console.log(state)
  }
}

main()