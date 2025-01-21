import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, subswh, subsf, all_projections, getRandomInt  } from './factory_protocol'
import { projectCombineMachines } from '@actyx/machine-check'

const qcr = Composition.makeMachine('QCR')

export const s0 = qcr.designEmpty('s0')
    .command("observe", [Events.observing], (s, _) => {
        console.log("began observing");
        return [Events.observing.make({})]
    })
    .finish()
export const s1 = qcr.designEmpty('s1').finish()
export const s2 = qcr.designState('s2').withPayload<{modelName: string, decision: string}>()
    .command("test", [Events.report], (s: any, _: any) => {
        console.log("the newly built", s.self.modelName, " is", s.self.decision);
        return [Events.report.make({modelName: s.self.modelName, decision: s.self.decision})]})
    .finish()

s0.react([Events.observing], s1, (_) => s1.make())
s1.react([Events.car], s2, (_, e) => {
    console.log("received a ", e.payload.modelName);
    if (e.payload.part !== 'broken part') { return s2.make({modelName: e.payload.modelName, decision: "ok"}) }
    else { return s2.make({ modelName: e.payload.modelName, decision: "notOk"}) }})

const result_projection = projectCombineMachines(interfacing_swarms, subs, "QCR")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data

const cMap = new Map()
cMap.set(Events.report.type, (s: any, _: any) => {
    console.log("the newly built", s.self.modelName, " is", s.self.decision);
    return [Events.report.make({modelName: s.self.modelName, decision: s.self.decision})]})
cMap.set(Events.observing.type, (s: any, _: any) => {
    console.log("began observing");
    return [Events.observing.make({})]})
const rMap = new Map()
const carReaction : ProjMachine.ReactionEntry = {
  genPayloadFun: (_, e) => {
    console.log("received a ", e.payload.modelName);
    if (e.payload.part !== 'broken part') { return {modelName: e.payload.modelName, decision: "ok"} }
    else { return {modelName: e.payload.modelName, decision: "notOk"} }}
}

rMap.set(Events.car.type, carReaction)
const fMap : any = {commands: cMap, reactions: rMap, initialPayloadType: undefined}
const [m3, i3] = Composition.extendMachine("QCR", projection, Events.allEvents, fMap)

async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunner(app, tags, i3, undefined)

    for await (const state of machine) {
      console.log("quality control robot. state is:", state.type)
      if (state.payload !== undefined) {
        console.log("state payload is:", state.payload)
      }
      console.log()
      const s = state.cast()
      for (var c in s.commands()) {
        if (c === 'observe') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1).includes('observe')) {
                    s1.observe()
                }
            }, getRandomInt(2000, 5000))
            break
        }
        if (c === 'test') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1).includes('test')) {
                    s1.test()
                }
            }, getRandomInt(4000, 8000))
            break
        }
      }
    }
    app.dispose()
}

main()