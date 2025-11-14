import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_protocols, protocol_1, subs_proto1, subs_composition, printState  } from './protocol'
import * as readline from 'readline';
import { checkComposedProjection } from '@actyx/machine-check';

const machineA = Composition.makeMachine('roleInterface')
export const s0 = machineA.designEmpty('s0').finish()
export const s1 = machineA.designEmpty('s1')
  .command('cmdI1', [Events.I1], () => [Events.I1.make({})])
  .finish()
export const s2 = machineA.designEmpty('s2')
  .command('cmdI2', [Events.I2], () => [Events.I2.make({})])
  .finish()
export const s3 = machineA.designEmpty('s3').finish()
export const s5 = machineA.designEmpty('s5').finish()

s0.react([Events.A], s1, () => s1.make())
s0.react([Events.B], s5, () => s5.make())
s1.react([Events.I1], s2, () => s2.make())
s2.react([Events.I2], s3, () => s3.make())

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(protocol_1, subs_proto1, "roleInterface", machineA.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapted machine
const [machineInterfaceAdapted, s0Adapted] = Composition.adaptMachine('roleInterface', interfacing_protocols, 0, subs_composition, [machineA, s0], true).data!

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('bt-experiment')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, machineInterfaceAdapted)
  printState(machineInterfaceAdapted.machineName, s0Adapted.mechanism.name, undefined)

  for await (const state of machine) {
    if (state.isLike(s1)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s1)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.cmdI1()
        }
      }, 3000)
    }
    if (state.isLike(s2)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s2)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.cmdI2()
        }
      }, 3000)
    }
  }

  app.dispose()
}

main()