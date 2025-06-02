import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_quality_protocol, subs_composition, quality_protocol, subs_quality, getRandomInt, print_event, printState  } from './protocol'
import { checkComposedProjection, projectionAndInformation, projectionAndInformationNew } from '@actyx/machine-check'
import chalk from 'chalk'
import * as readline from 'readline';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

// Using the machine runner DSL an implmentation of quality control robot in Gquality is:
const qcr = Composition.makeMachine('QCR')
export const s0 = qcr.designEmpty('s0')
    .command("observe", [Events.observing], (s) => {
        return [Events.observing.make({})]
    })
    .finish()
export const s1 = qcr.designEmpty('s1').finish()
export const s2 = qcr.designState('s2').withPayload<{modelName: string, decision: string}>()
    .command("test", [Events.report], (s: any) => {
        return [Events.report.make({modelName: s.self.modelName, decision: s.self.decision})]})
    .finish()
export const s3 = qcr.designEmpty('s3').finish()

s0.react([Events.observing], s1, (_, e) => { return s1.make() })
s1.react([Events.car], s2, (_, e) => {
    if (e.payload.part !== 'broken part') { return s2.make({modelName: e.payload.modelName, decision: "ok"}) }
    else { return s2.make({ modelName: e.payload.modelName, decision: "notOk"}) }})
s2.react([Events.report], s3, (_, e) => { return s3.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(quality_protocol, subs_quality, "QCR", qcr.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Projection of warehouse || factory || quality over QCR
const projectionInfoResult = projectionAndInformation(warehouse_factory_quality_protocol, subs_composition, "QCR")
//const projectionInfoResult = projectionAndInformationNew(warehouse_factory_quality_protocol, subs_composition, "QCR", qcr.createJSONForAnalysis(s0), 2)

if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data
//console.log(JSON.stringify(projectionInfo1, null, 2))

// Adapted  machine
const [qcrAdapted, s0Adapted] = Composition.adaptMachine("Quality Control", projectionInfo, Events.allEvents, s0, true)

// Run the extended machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-factory-quality')
    const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)
    printState(qcrAdapted.machineName, s0Adapted.mechanism.name, undefined)
    console.log(chalk.bgBlack.red.dim`    obs!`);

    for await (const state of machine) {
        if(state.isLike(s0)) {
            rl.on('line', (_) => {
            const stateAfterTimeOut = machine.get()
            if (stateAfterTimeOut?.isLike(s0)) {
                stateAfterTimeOut?.cast().commands()?.observe()
            }
            })
        }

        if(state.isLike(s2)) {
        rl.on('line', (_) => {
          const stateAfterTimeOut = machine.get()
          if (stateAfterTimeOut?.isLike(s2)) {
            stateAfterTimeOut?.cast().commands()?.test()
          }
        })
        }
    }
    app.dispose()
}

main()