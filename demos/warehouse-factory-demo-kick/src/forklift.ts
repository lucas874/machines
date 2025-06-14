import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_protocol, subs_composition, getRandomInt, warehouse_protocol, subs_warehouse, print_event } from './protocol'
import { checkComposedProjection, projectionAndInformation } from '@actyx/machine-check'

const forkliftFinal = "{ { { 3 } } || { { 0 } }, { { 3 } } || { { 2 } } }"

// Using the machine runner DSL an implmentation of forklift in the warehouse protocol w.r.t. subs_warehouse is:
const forklift = Composition.makeMachine('FL')
export const s0 = forklift.designEmpty('s0') .finish()
export const s1 = forklift.designState('s1').withPayload<{id: string}>()
  .command('get', [Events.pos], (state: any) => {
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

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(warehouse_protocol, subs_warehouse, "FL", forklift.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Projection of warehouse || factory over FL
const projectionInfoResult = projectionAndInformation(warehouse_factory_protocol, subs_composition, "FL")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

// Adapted machine
const [forkliftAdapted, s0_] = Composition.adaptMachine("FL", projectionInfo, Events.allEvents, s0)

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-factory')
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
      if(state.isLike(s1)) {
        setTimeout(() => {
          const stateAfterTimeOut = machine.get()
          if (stateAfterTimeOut?.isLike(s1)) {
            stateAfterTimeOut?.cast().commands()?.get()
          }
        }, getRandomInt(4000, 8000))
      }
    }
    app.dispose()
}

main()