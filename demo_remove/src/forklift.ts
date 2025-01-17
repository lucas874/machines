import { Actyx } from '@actyx/sdk'
import { createMachineRunner } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, subswh, subsf, all_projections, getRandomInt } from './factory_protocol'
import { projectCombineMachines } from '@actyx/machine-check'
import { MachineAnalysisResource } from '@actyx/machine-runner/lib/esm/design/protocol'
/* type partIDToGet = {
    pid: string
} */
const forklift = Composition.makeMachine('FL')
export const s0 = forklift.designEmpty('s0') .finish()
export const s1 = forklift.designEmpty('s1')
    .command('get', [Events.position], () => [{}])
    .finish()
export const s2 = forklift.designEmpty('s2').finish()
/* console.log("sub comp: ", JSON.stringify(subs))
console.log("sub wh: ", JSON.stringify(subswh))
console.log("sub f: ", JSON.stringify(subsf)) */



s0.react([Events.partID], s1, (_) => s1.make())
s1.react([Events.position], s0, (_) => s0.make())
s0.react([Events.time], s2, (_) => s2.make())

const result_projection = projectCombineMachines(interfacing_swarms, subs, "FL")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data

const cMap = new Map()
const rMap = new Map()
const statePayloadMap = new Map()
const fMap : any = {commands: cMap, reactions: rMap, statePayloads: statePayloadMap}
const mAnalysisResource: MachineAnalysisResource = {initial: projection.initial, subscriptions: [], transitions: projection.transitions}
const [m3, i3] = Composition.extendMachine("FL", mAnalysisResource, Events.allEvents, fMap)
//console.log(m3.createJSONForAnalysis(i3))

async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunner(app, tags, i3, undefined)
    //var hasRequested = false
    //var isDone = false
    for await (const state of machine) {
      console.log("state is: ", state)
      /* if (isDone) {
          console.log("shutting down")
          break
      } */

      const s = state.cast()
      for (var c in s.commands()) {
          var cmds = s.commands() as any;
          if (c === 'get') {
            setTimeout(() => {
                cmds?.get()
            }, getRandomInt(2000, 5000))
            break
          }
      }
    }
    app.dispose()
}

main()