import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_protocols, protocol_2, subs_proto2, subs_composition, printState  } from './protocol'
import * as readline from 'readline';
import { checkComposedProjection } from '@actyx/machine-check';

const machineInterfaceProto1 = Composition.makeMachine('roleInterface')
export const s0 = machineInterfaceProto1.designEmpty('s0')
  .command('cmdI1', [Events.I1], () => [Events.I1.make({})])
  .finish()
export const s1 = machineInterfaceProto1.designEmpty('s1').finish()
export const s2 = machineInterfaceProto1.designEmpty('s2')
  .command('cmdI2', [Events.I2], () => [Events.I2.make({})])
  .finish()
export const s3 = machineInterfaceProto1.designEmpty('s3').finish()
export const s4 = machineInterfaceProto1.designEmpty('s4').finish()

s0.react([Events.I1], s1, () => s1.make())
s1.react([Events.E], s4, () => s4.make())
s1.react([Events.D], s2, () => s2.make())
s2.react([Events.I2], s3, () => s3.make())

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(protocol_2, subs_proto2, "roleInterface", machineInterfaceProto1.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapted machine
const [machineInterfaceAdapted, s0Adapted] = Composition.adaptMachine('roleInterface', interfacing_protocols, 1, subs_composition, [machineInterfaceProto1, s0], true).data!

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('bt-experiment')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, machineInterfaceAdapted)
  printState(machineInterfaceAdapted.machineName, s0Adapted.mechanism.name, undefined)

  for await (const state of machine) {
    if (state.isLike(s0)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s0)) {
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