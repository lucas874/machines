import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_protocols, subs_composition, getRandomInt, protocol_2, subs_proto2, printState } from './protocol'
import { checkComposedProjection } from '@actyx/machine-check';

const machineD = Composition.makeMachine('roleD')
export const s0 = machineD.designEmpty('s0').finish()
export const s1 = machineD.designEmpty('s1')
  .command('cmdE', [Events.E], () => [Events.E.make({})])
  .command('cmdD', [Events.D], () => [Events.D.make({})])
  .finish()
export const s2 = machineD.designEmpty('s2').finish()
export const s4 = machineD.designEmpty('s3').finish()

s0.react([Events.I1], s1, () => s1.make())
s1.react([Events.E], s4, () => s4.make())
s1.react([Events.D], s2, () => s2.make())

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(protocol_2, subs_proto2, "roleD", machineD.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapted machine
const [machineDAdapted, s0Adapted] = Composition.adaptMachine('roleD', interfacing_protocols, 0, subs_composition, [machineD, s0], true).data!

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('bt-experiment')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, machineDAdapted)
  printState(machineDAdapted.machineName, s0Adapted.mechanism.name, undefined)

  for await (const state of machine) {
    if (state.isLike(s1)) {
      setTimeout(() => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s1)) {
          console.log()
          stateAfterTimeOut?.cast().commands()?.cmdD()
        }
      }, 3000)
    }
  }

  app.dispose()
}

main()