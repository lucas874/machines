import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, subswh, subsf, all_projections, getRandomInt } from './factory_protocol'
import { projectCombineMachines } from '@actyx/machine-check'
import { MachineAnalysisResource } from '@actyx/machine-runner/lib/esm/design/protocol'

const forklift = Composition.makeMachine('FL')
export const s0 = forklift.designEmpty('s0') .finish()
export const s1 = forklift.designEmpty('s1')
    .command('get', [Events.position], () => { return [Events.position.make({position: "x", part: "lars"})]})
    .finish()
export const s2 = forklift.designEmpty('s2').finish()

s0.react([Events.partID], s1, (_) => s1.make())
s1.react([Events.position], s0, (_) => s0.make())
s0.react([Events.time], s2, (_) => s2.make())

const result_projection = projectCombineMachines(interfacing_swarms, subs, "FL")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data

const cMap = new Map()
cMap.set(Events.position.type, (state: any, _: any) => {console.log("retrieved a ", state.self.id, " at x"); return [Events.position.make({position: "x", part: state.self.id})]})

const rMap = new Map()
const partIDReaction : ProjMachine.ReactionEntry = {
  identifiedByInput: true,
  genPayloadFun: (_, e) => { console.log("a ", e.payload.id, "was requested"); return {id: e.payload.id} }//return {lastMl: 100, totalMl: 100} }
}
rMap.set(Events.partID.type, partIDReaction)
const fMap : any = {commands: cMap, reactions: rMap}
const mAnalysisResource: MachineAnalysisResource = {initial: projection.initial, subscriptions: [], transitions: projection.transitions}
const [m3, i3] = Composition.extendMachine("FL", mAnalysisResource, Events.allEvents, fMap)

async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunner(app, tags, i3, undefined)

    for await (const state of machine) {
      console.log("state is: ", state)

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