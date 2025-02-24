import { Actyx } from '@actyx/sdk'
import { createMachineRunner } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, getRandomInt, all_projections } from './warehouse_protocol'
import { projectCombineMachines, checkComposedProjection } from '@actyx/machine-check'

/*

Using the machine runner DSL an implmentation of door in Gwarehouse is:

const door = Composition.makeMachine('D')
export const s0 = door.designEmpty('s0')
    .command('close', [Events.time], () => {
        var dateString = new Date().toLocaleString();
        console.log("closed warehouse at:", dateString);
        return [Events.time.make({timeOfDay: dateString})]})
    .finish()
export const s1 = door.designEmpty('s1').finish()
export const s2 = door.designEmpty('s2').finish()

s0.react([Events.partID], s1, (_) => s1.make())
s1.react([Events.part], s0, (_) => s0.make())
s0.react([Events.time], s2, (_) => s2.make())
*/
/* for (var p of all_projections) {
    console.log(JSON.stringify(p))
    console.log("$$$$")
} */

// Projection of Gwarehouse || Gfactory || Gquality over D
const result_projection = projectCombineMachines(interfacing_swarms, subs, "D")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data

// Command map
const cMap = new Map()
cMap.set(Events.time.type, () => {
    var dateString = new Date().toLocaleString();
    console.log("closed warehouse at:", dateString);
    return [Events.time.make({timeOfDay: dateString})]})

// Reaction map
const rMap = new Map()
const fMap : any = {commands: cMap, reactions: rMap, initialPayloadType: undefined}

// Extended machine
const [m3, i3] = Composition.extendMachine("D", projection, Events.allEvents, fMap)
const checkProjResult = checkComposedProjection(interfacing_swarms, subs, "D", m3.createJSONForAnalysis(i3))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Run the extended machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunner(app, tags, i3, undefined)

    for await (const state of machine) {
      console.log("door. state is:", state.type)
      if (state.payload !== undefined) {
        console.log("state payload is:", state.payload)
      }
      console.log()
      const s = state.cast()
      for (var c in s.commands()) {
          if (c === 'close') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1).includes('close')) {
                    s1.close()
                }
            }, getRandomInt(5000, 8000))
            break
          }
      }
    }
    app.dispose()
}

main()