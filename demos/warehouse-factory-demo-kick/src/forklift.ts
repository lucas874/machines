import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, getRandomInt, print_event } from './protocol'
import { checkComposedProjection, projectionAndInformation } from '@actyx/machine-check'

const forkliftFinal = "{ { { 3 } } || { { 0 } }, { { 3 } } || { { 2 } } }"

// Using the machine runner DSL an implmentation of forklift in Gwarehouse is:
const forklift = Composition.makeMachine('FL')
export const s0 = forklift.designEmpty('s0') .finish()
export const s1 = forklift.designState('s1').withPayload<{id: string}>()
  .command('get', [Events.pos], (state: any, _: any) => {
    console.log("retrieved a", state.self.id, "at position x");
    return [Events.pos.make({position: "x", part: state.self.id})]})
  .finish()
export const s2 = forklift.designEmpty('s2').finish()

s0.react([Events.partReq], s1, (_, e) => {
    print_event(e);
    console.log("a", e.payload.id, "was requested");
    if (getRandomInt(0, 10) >= 9) { return { id: "broken part" } }
    return s1.make({id: e.payload.id}) })
s1.react([Events.pos], s0, (_, e) => { print_event(e); return s0.make() })
s0.react([Events.closingTime], s2, (_, e) => { print_event(e); return s2.make() })

// Projection of Gwarehouse || Gfactory || Gquality over FL
const projectionInfoResult = projectionAndInformation(interfacing_swarms, subs, "FL")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

// Adapted machine
const [forkliftAdapted, s0_] = Composition.adaptMachine("FL", projectionInfo, Events.allEvents, s0)
const checkProjResult = checkComposedProjection(interfacing_swarms, subs, "FL", forkliftAdapted.createJSONForAnalysis(s0_))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunnerBT(app, tags, s0_, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)

    for await (const state of machine) {
      console.log("Forklift. State is:", state.type)
      if (state.payload !== undefined) {
        console.log("State payload is:", state.payload)
      }
      console.log()
      if (state.type === forkliftFinal) {
        console.log("\x1b[32mForklift reached its final state. Press CTRL + C to exit.\x1b[0m")
      }
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