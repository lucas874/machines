import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT} from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_protocols, subs_composition, protocol_1, subs_proto1, printState, getRandomInt } from './protocol'
import * as readline from 'readline';
import chalk from "chalk";
import { checkComposedProjection } from '@actyx/machine-check';

const machineA = Composition.makeMachine('roleA')
export const s0 = machineA.designEmpty('s0')
    .command('cmdA', [Events.A], () => [Events.A.make({})])
    .command('cmdB', [Events.B], () => [Events.B.make({})])
    .finish()
export const s1 = machineA.designEmpty('s1').finish()
export const s2 = machineA.designEmpty('s2').finish()
export const s3 = machineA.designEmpty('s3')
  .command('cmdC', [Events.C], () => [Events.C.make({})])
  .finish()
export const s4 = machineA.designEmpty('s4').finish()
export const s5 = machineA.designEmpty('s5').finish()

s0.react([Events.A], s1, () => s1.make())
s0.react([Events.B], s5, () => s5.make())
s1.react([Events.I1], s2, () => s2.make())
s2.react([Events.I2], s3, () => s3.make())
s3.react([Events.C], s4, () => s4.make())

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(protocol_1, subs_proto1, "roleA", machineA.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapted machine
const [machineAAdapted, s0Adapted] = Composition.adaptMachine('roleA', interfacing_protocols, 0, subs_composition, [machineA, s0], true).data!

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('bt-experiment')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, machineAAdapted)
  printState(machineAAdapted.machineName, s0Adapted.mechanism.name, undefined)
  console.log(chalk.bgBlack.red.dim`    A!`);
  console.log(chalk.bgBlack.red.dim`    B!`);

  for await (const state of machine) {
    if (state.isLike(s0)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s0)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.cmdA()
        }
      }, 3000)
    }
    if (state.isLike(s3)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s3)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.cmdC()
        }
      }, 3000)
    }
  }

  app.dispose()
}

main()