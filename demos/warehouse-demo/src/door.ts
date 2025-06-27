import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_protocol, subs_warehouse, printState, getRandomInt } from './protocol'
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
  .command('close', [Events.time], () => {
    //var dateString = new Date().toLocaleString();
    //return [Events.time.make({timeOfDay: new Date().toLocaleString() })]})
    return [Events.time.make({ timeOfDay: new Date().toString() })]
  }).finish()
export const s1 = door.designEmpty('s1').finish()
export const s2 = door.designEmpty('s2').finish()

s0.react([Events.partID], s1, (_, e) => { return s1.make() })
s1.react([Events.part], s0, (_, e) => { return s0.make() })
s0.react([Events.time], s2, (_, e) => { return s2.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(warehouse_protocol, subs_warehouse, "D", door.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [doorAdapted, s0Adapted] = Composition.adaptMachine('D', warehouse_protocol, 0, subs_warehouse, [door, s0], true).data!

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('warehouse')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, doorAdapted)
  printState(doorAdapted.machineName, s0Adapted.mechanism.name, undefined)
  console.log(chalk.bgBlack.red.dim`    time!`);

  for await (const state of machine) {
    if (state.isLike(s0)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s0)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.close()
        }
      }, getRandomInt(3000, 8000))
    }
  }
  rl.close();
  app.dispose()
}

main()