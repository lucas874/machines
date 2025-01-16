import { Actyx } from '@actyx/sdk'
import { createMachineRunner } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, all_projections, getRandomInt } from './factory_protocol'
import { projectCombineMachines } from '@actyx/machine-check'
import { MachineAnalysisResource } from '@actyx/machine-runner/lib/esm/design/protocol'

/* for (var p of all_projections) {
    console.log(JSON.stringify(p))
} */

const door = Composition.makeMachine('D')
export const s0 = door.designEmpty('s0')
    .command('close', [Events.time], () => {var dateString = new Date().toLocaleString(); console.log(dateString); return [Events.time.make({timeOfDay: dateString})]})
    .finish()
export const s1 = door.designEmpty('s1').finish()
export const s2 = door.designEmpty('s2').finish()

s0.react([Events.partID], s1, (_) => s1.make())
s1.react([Events.part], s0, (_) => s0.make())
s0.react([Events.time], s2, (_) => s2.make())

const result_projection = projectCombineMachines(interfacing_swarms, subs, "D")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data

const cMap = new Map()
cMap.set(Events.time.type, () => {var dateString = new Date().toLocaleString(); console.log(dateString); return [Events.time.make({timeOfDay: dateString})]})
const rMap = new Map()
const statePayloadMap = new Map()
const fMap : any = {commands: cMap, reactions: rMap, statePayloads: statePayloadMap}
const mAnalysisResource: MachineAnalysisResource = {initial: projection.initial, subscriptions: [], transitions: projection.transitions}
const [m3, i3] = Composition.extendMachine("D", mAnalysisResource, Events.allEvents, [door, s0], fMap)
//console.log(m3.createJSONForAnalysis(i3))
//console.log(getRandomInt(2000, 5000))

async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunner(app, tags, i3, undefined)

    for await (const state of machine) {
      console.log("state is: ", state)
      const s = state.cast()
      for (var c in s.commands()) {
          var cmds = s.commands() as any;
          if (c === 'close') {
            setTimeout(() => {
                const canClose = machine.get()?.commandsAvailable()
                if (canClose) {
                    var s1 = machine.get()?.cast()?.commands() as any
                    s1.close()
                }




                /* if ((await machine.peekNext()).done) {
                    console.log("done")
                    cmds?.close()
                } else {
                    console.log("not done")
                } */
                /* const whenOn = (await machine.peekNext()).done?//.as(peekValue)
                if (whenOn) {
                    cmds?.close()
                } */


            }, getRandomInt(2000, 5000))
            break
          }
      }
    }
    app.dispose()
}

main()