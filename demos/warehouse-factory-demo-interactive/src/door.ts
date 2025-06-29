import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT} from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouseFactoryProtocols, subsComposition, warehouseProtocol, subsWarehouse, printState } from './protocol'
import * as readline from 'readline';
import chalk from "chalk";
import { checkComposedProjection } from '@actyx/machine-check';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

// Using the machine runner DSL an implmentation of door in warehouse w.r.t. subs_warehouse is:
const door = Composition.makeMachine('Door')
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
const checkProjResult = checkComposedProjection(warehouseProtocol, subsWarehouse, "D", door.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapted machine
const [doorAdapted, s0Adapted] = Composition.adaptMachine('D', warehouseFactoryProtocols, 0, subsComposition, [door, s0], true).data!

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('warehouse-factory')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, doorAdapted)
  printState(doorAdapted.machineName, s0Adapted.mechanism.name, undefined)
  console.log(chalk.bgBlack.red.dim`    ${Events.closingTime.type}!`);

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
  rl.close();
  app.dispose()
}

main()