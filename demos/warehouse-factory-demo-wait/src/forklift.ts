import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_protocol, subs_composition, getRandomInt, warehouse_protocol, subs_warehouse, print_event, printState } from './protocol'
import { checkComposedProjection, projectionAndInformation, projectionAndInformationNew } from '@actyx/machine-check'
import * as readline from 'readline';
import chalk from "chalk";

const log = console.log
const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

// Using the machine runner DSL an implmentation of forklift in the warehouse protocol w.r.t. subs_warehouse is:
const forklift = Composition.makeMachine('Forklift')
export const s0 = forklift.designEmpty('s0') .finish()
export const s1 = forklift.designState('s1').withPayload<{partName: string}>()
  .command('get', [Events.pos], (state: any) => {
    //console.log("retrieved a", state.self.partName, "at position x");
    //readline.moveCursor(process.stdout, 0, -2);
    //readline.clearScreenDown(process.stdout);
    //log(chalk.green.bold`    pos! ➡ \{"position":"x","partName":${state.self.partName},"type":"pos"\}`);
    return [Events.pos.make({position: "x", partName: state.self.partName})]})
  .finish()
export const s2 = forklift.designEmpty('s2').finish()

s0.react([Events.partID], s1, (_, e) => {
    //console.log("a", e.payload.partName, "was requested");
    if (getRandomInt(0, 10) >= 9) { return { partName: "broken part" } }
    return s1.make({partName: e.payload.partName}) })
s1.react([Events.pos], s0, (_, e) => { return s0.make() })
s0.react([Events.time], s2, (_, e) => { return s2.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(warehouse_protocol, subs_warehouse, "FL", forklift.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Projection of warehouse || factory over FL
const projectionInfoResult = projectionAndInformation(warehouse_factory_protocol, subs_composition, "FL")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

// Adapted machine
const [forkliftAdapted, s0Adapted] = Composition.adaptMachine('Forklift', projectionInfo, Events.allEvents, s0, true)

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('warehouse-factory-quality')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)
  printState(forkliftAdapted.machineName, s0Adapted.mechanism.name, undefined)

  for await (const state of machine) {
    //log(chalk.blue`State: ${state.type}. Payload: ${state.payload === undefined ? "{}" : JSON.stringify(state.payload, null, 0) }`)

    if(state.isLike(s1)) {
      //log(chalk.red`    pos!`);
        rl.on('line', (_) => {
          const stateAfterTimeOut = machine.get()
          if (stateAfterTimeOut?.isLike(s1)) {
            stateAfterTimeOut?.cast().commands()?.get()
          }
        })
    }
  }
  rl.close();
  app.dispose()
}

main()