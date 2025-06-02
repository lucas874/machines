import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT} from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_quality_protocol, subs_composition, getRandomInt, warehouse_protocol, subs_warehouse, print_event, printState } from './protocol'
import { checkComposedProjection, projectionAndInformation, projectionAndInformationNew } from '@actyx/machine-check'
import chalk from 'chalk'
import * as readline from 'readline';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

// Using the machine runner DSL an implmentation of door in warehouse w.r.t. subs_warehouse is:
const door = Composition.makeMachine('D')
export const s0 = door.designEmpty('s0')
    .command('close', [Events.closingTime], () => {
        var dateString = new Date().toLocaleString();
        return [Events.closingTime.make({timeOfDay: dateString})]})
    .finish()
export const s1 = door.designEmpty('s1').finish()
export const s2 = door.designEmpty('s2').finish()

s0.react([Events.partReq], s1, (_, e) => { return s1.make() })
s1.react([Events.partOK], s0, (_, e) => { return s0.make() })
s0.react([Events.closingTime], s2, (_, e) => { return s2.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(warehouse_protocol, subs_warehouse, "D", door.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Projection of warehouse || factory || quality over D
//const projectionInfoResult = projectionAndInformationNew(warehouse_factory_quality_protocol, subs_composition, "D", door.createJSONForAnalysis(s0), 0)
const projectionInfoResult = projectionAndInformation(warehouse_factory_quality_protocol, subs_composition, "D")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

// Adapt machine
//const [doorAdapted, s0_] = Composition.adaptMachineNew("D", projectionInfo, Events.allEvents, s0)
const [doorAdapted, s0Adapted] = Composition.adaptMachine("Door", projectionInfo, Events.allEvents, s0, true)
//console.log(JSON.stringify(subs_composition, null, 2))

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-factory-quality')
    const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)
    printState(doorAdapted.machineName, s0Adapted.mechanism.name, undefined)
    console.log(chalk.bgBlack.red.dim`    time!`);

    for await (const state of machine) {
      if (state.isLike(s0)) {
        rl.on('line', (_) => {
          const stateAfterTimeOut = machine.get()
          if (stateAfterTimeOut?.isLike(s0)) {
            stateAfterTimeOut?.cast().commands()?.close()
          }
        })
      }
    }
    app.dispose()
}

main()