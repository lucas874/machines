import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, getRandomInt  } from './protocol'
import { checkComposedProjection, projectionAndInformation } from '@actyx/machine-check'

// Using the machine runner DSL an implmentation of quality control robot in Gquality is:
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

// Projection of Gwarehouse || Gfactory || Gquality over QCR
const projectionInfoResult = projectionAndInformation(interfacing_swarms, subs, "QCR")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

// Extended machine
const [qcrAdapted, s0_] = Composition.adaptMachine("QCR", projectionInfo, Events.allEvents, s0)
const checkProjResult = checkComposedProjection(interfacing_swarms, subs, "QCR", qcrAdapted.createJSONForAnalysis(s0_))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Run the extended machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunnerBT(app, tags, s0_, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)

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
                if (Object.keys(s1 || {}).includes('observe')) {
                    s1.observe()
                }
            }, getRandomInt(2000, 5000))
            break
        }
        if (c === 'test') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1 || {}).includes('test')) {
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