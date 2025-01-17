import { createMachineRunner } from '@actyx/machine-runner'
import { Actyx } from '@actyx/sdk'
import { Events, manifest, protocol } from './protocol'

type SpentWater = {
  lastMl: number
  totalMl: number
}

const machine = protocol.makeMachine('robot')

export const Idle = machine.designState('Idle').withPayload<SpentWater>().finish()

export const WateringPlant = machine.designState('WateringPlant').withPayload<SpentWater>().finish()

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

export async function main() {
  const sdk = await Actyx.of(manifest)
  const tags = protocol.tagWithEntityId('robot-1')
  const machine = createMachineRunner(sdk, tags, Idle, {
    lastMl: 0,
    totalMl: 0,
  })

  for await (const state of machine) {
    console.log(state)
  }
}

main()