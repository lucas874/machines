import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_quality_protocol, subs_composition, quality_protocol, subs_quality, getRandomInt, print_event  } from './protocol'
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
export const s3 = qcr.designEmpty('s3').finish()

s0.react([Events.observing], s1, (_, e) => { print_event(e); return s1.make() })
s1.react([Events.car], s2, (_, e) => {
    print_event(e);
    console.log("received a ", e.payload.modelName);
    if (e.payload.part !== 'broken part') { return s2.make({modelName: e.payload.modelName, decision: "ok"}) }
    else { return s2.make({ modelName: e.payload.modelName, decision: "notOk"}) }})
s2.react([Events.report], s3, (_, e) => { print_event(e); return s3.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(quality_protocol, subs_quality, "QCR", qcr.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Projection of warehouse || factory || quality over QCR
const projectionInfoResult = projectionAndInformation(warehouse_factory_quality_protocol, subs_composition, "QCR")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

// Adapted  machine
const [qcrAdapted, s0_] = Composition.adaptMachine("QCR", projectionInfo, Events.allEvents, s0)

// Run the extended machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-factory-quality')
    const machine = createMachineRunnerBT(app, tags, s0_, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)

    for await (const state of machine) {
      console.log("Quality control robot. State is:", state.type)
      if (state.payload !== undefined) {
        console.log("State payload is:", state.payload)
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