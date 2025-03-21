import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine, createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, getRandomInt } from './factory_protocol'
import { projectCombineMachines, checkComposedProjection, projectionAndInformation } from '@actyx/machine-check'

// Using the machine runner DSL an implmentation of forklift in Gwarehouse is:
const forklift = Composition.makeMachine('FL')
export const s0 = forklift.designEmpty('s0') .finish()
export const s1 = forklift.designState('s1').withPayload<{id: string}>()
  .command('get', [Events.position], (state: any, _: any) => {
    console.log("retrieved a", state.self.id, "at position x");
    return [Events.position.make({position: "x", part: state.self.id})]})
  .finish()
export const s2 = forklift.designEmpty('s2').finish()

s0.react([Events.partID], s1, (_, e) => {
    console.log("a", e.payload.id, "was requested");
    if (getRandomInt(0, 10) >= 9) { return { id: "broken part" } }
    return s1.make({id: e.payload.id}) })
s1.react([Events.position], s0, (_) => s0.make())
s0.react([Events.time], s2, (_) => s2.make())

// Projection of Gwarehouse || Gfactory || Gquality over FL
const projectionInfoResult = projectionAndInformation(interfacing_swarms, subs, "FL")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data
//console.log(projectionInfo)

// Adapted machine
const [forkliftAdapted, s0_] = Composition.adaptMachine("FL", projectionInfo, Events.allEvents, s0)
const checkProjResult = checkComposedProjection(interfacing_swarms, subs, "FL", forkliftAdapted.createJSONForAnalysis(s0_))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    //const machine = createMachineRunner(app, tags, s0, undefined)
    const machine = createMachineRunnerBT(app, tags, s0_, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)

    for await (const state of machine) {
      console.log("forklift. state is:", state.type)
      if (state.payload !== undefined) {
        console.log("state payload is:", state.payload)
      }
      console.log()
      const s = state.cast()
      for (var c in s.commands()) {
          if (c === 'get') {
            setTimeout(() => {
              var s1 = machine.get()?.cast()?.commands() as any
              if (Object.keys(s1 || {}).includes('get')) {
                s1.get()
              }
            }, 1500)
            break
          }
      }
    }
    app.dispose()
}

main()