import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine, createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, getRandomInt, loopThroughJSON } from './warehouse_protocol'
import { projectCombineMachines, checkComposedProjection } from '@actyx/machine-check'
//import { createMachineRunnerBT } from '@actyx/machine-runner/lib/esm/runner/runner'

/*

Using the machine runner DSL an implmentation of forklift in Gwarehouse is:

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
*/

// With our extension of the library we create a map from events to reactions
// and commands instead and use the projection of the composition over
// the role to create the extended machine

// Projection of Gwarehouse || Gfactory || Gquality over FL
const result_projection = projectCombineMachines(interfacing_swarms, subs, "FL")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data
console.log(projection)
// Command map
const cMap = new Map()
cMap.set(Events.position.type, (state: any, _: any) => {
  console.log("retrieved a", state.self.id, "at position x");
  return {position: "x", part: state.self.id}})

// Reaction map
const rMap = new Map()
const partIDReaction : ProjMachine.ReactionEntry = {
  genPayloadFun: (s, e) => {
    console.log("a", e.payload.id, "was requested");
    if (getRandomInt(0, 10) >= 9) { return { id: "broken part" } }
    return {id: e.payload.id} }
}
rMap.set(Events.partID.type, partIDReaction)
const fMap : any = {commands: cMap, reactions: rMap, initialPayloadType: undefined}

// Extended machine
const [m3, i3] = Composition.extendMachineBT("FL", projection, Events.allEvents, fMap, new Set<string>([Events.partID.type, Events.time.type]))
const checkProjResult = checkComposedProjection(interfacing_swarms, subs, "FL", m3.createJSONForAnalysis(i3))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

  // Run the extended machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunnerBT(app, tags, i3, undefined)

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