import { Actyx } from '@actyx/sdk'
import { createMachineRunner } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, all_projections  } from './factory_protocol'
import { projectCombineMachines } from '@actyx/machine-check'
import { MachineAnalysisResource } from '@actyx/machine-runner/lib/esm/design/protocol'

//import { protocol } from './protocol'

for (var p of all_projections) {
    console.log(JSON.stringify(p))
}
//new Date().toLocaleString()
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
console.log(m3.createJSONForAnalysis(i3))