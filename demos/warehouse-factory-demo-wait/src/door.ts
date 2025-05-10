import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT} from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_protocol, subs_composition, getRandomInt, warehouse_protocol, subs_warehouse, print_event, printState } from './protocol'
import { checkComposedProjection, projectionAndInformation } from '@actyx/machine-check'
import * as readline from 'readline';
import chalk from "chalk";

const log = console.log;

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

// Using the machine runner DSL an implmentation of door in warehouse w.r.t. subs_warehouse is:
const door = Composition.makeMachine('Door')
export const s0 = door.designEmpty('s0')
    .command('close', [Events.time], () => {
        var dateString = new Date().toLocaleString();
        //readline.moveCursor(process.stdout, 0, -2);
        //readline.clearScreenDown(process.stdout);
        //log(chalk.green.bold`    time! ➡ \{"timeOfDay":"${dateString}","type":"time"\}`);
        return [Events.time.make({timeOfDay: dateString})]})
    .finish()
export const s1 = door.designEmpty('s1').finish()
export const s2 = door.designEmpty('s2').finish()

s0.react([Events.partID], s1, (_, e) => { return s1.make() })
s1.react([Events.part], s0, (_, e) => { return s0.make() })
s0.react([Events.time], s2, (_, e) => { return s2.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(warehouse_protocol, subs_warehouse, "D", door.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Projection of warehouse || factory over D
const projectionInfoResult = projectionAndInformation(warehouse_factory_protocol, subs_composition, "D")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

// Adapted machine
const [doorAdapted, s0Adapted] = Composition.adaptMachine('Door', projectionInfo, Events.allEvents, s0, true)

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('warehouse-factory-quality')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)
  printState(doorAdapted.machineName, s0Adapted.mechanism.name, undefined)
  log(chalk.red.dim`    time!`);

  for await (const state of machine) {
    //log(chalk.blue`State: ${state.type}. Payload: ${state.payload === undefined ? "{}" : JSON.stringify(state.payload, null, 0) }`)

    if (state.isLike(s0)) {
      //log(chalk.red`    time!`);
      rl.on('line', (_) => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s0)) {
          stateAfterTimeOut?.cast().commands()?.close()
        }
      })
    }
  }
  rl.close();
  app.dispose()
}

main()