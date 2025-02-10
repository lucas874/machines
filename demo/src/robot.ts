import { createMachineRunner, ProjMachine, ReactionContext } from '@actyx/machine-runner'
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
const needsWaterReaction : ProjMachine.ReactionEntry = {
  genPayloadFun:
    (state: any, event: any) => {
      console.log(`The plant is requesting ${event.payload.requiredWaterMl} ml of water!`)
      const newStatePayload = {
        lastMl: event.payload.requiredWaterMl + 5001,
        totalMl: state.self.totalMl + event.payload.requiredWaterMl + 5000,
      }
      //console.log("heeeeY ", state, event, "dsad")
      console.log(`Total water consumption: ${newStatePayload.totalMl}`)
      console.log("new state payload: ", newStatePayload)
      return newStatePayload
  }
}
const hasWaterReaction : ProjMachine.ReactionEntry = {
  genPayloadFun: (state, _: any) => { console.log("hej hej in fun ", state.self); console.log("hejj in funnn"); return state.self }//return {lastMl: 100, totalMl: 100} }
}
/* const needsWaterReaction = (state: any, event: any) => {
    console.log(`The plant is requesting ${event.payload.requiredWaterMl} ml of water!`)
    const newStatePayload = {
      lastMl: event.payload.requiredWaterMl,
      totalMl: state.self.totalMl + event.payload.requiredWaterMl,
    }
    console.log(`Total water consumption: ${newStatePayload.totalMl}`)
    return newStatePayload
} */
statePayloadMap.set(Events.NeedsWater.type, needsWaterReaction)
statePayloadMap.set(Events.HasWater.type, hasWaterReaction)
const fMap : any = {commands: cMap, reactions: statePayloadMap, initialPayloadType: undefined}
//const [m3, i3] = protocol.extendMachine("robot", m, Events.All, [machine, Idle], fMap)
const [m3, i3] = protocol.extendMachine("robot", m, Events.All, fMap)
export async function main() {
  const sdk = await Actyx.of(manifest)
  const tags = protocol.tagWithEntityId('robot-1')

  //const machine = createMachineRunner(sdk, tags, i2, undefined)
  const machine = createMachineRunner(sdk, tags, i3, {
    lastMl: 0,
    totalMl: 0,
  })
  for await (const state of machine) {
    console.log(state)
  }
}

main()