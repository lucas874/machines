import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouseFactoryProtocol, subsComposition, getRandomInt, warehouseProtocol, subsWarehouse, printState } from './protocol'
import * as readline from 'readline';
import { checkComposedProjection } from '@actyx/machine-check';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

// Using the machine runner DSL an implmentation of forklift in the warehouse protocol w.r.t. subs_warehouse is:
const forklift = Composition.makeMachine('Forklift')
export const s0 = forklift.designEmpty('s0') .finish()
export const s1 = forklift.designState('s1').withPayload<{partName: string}>()
  .command('get', [Events.pos], (state: any) => {
    return [Events.pos.make({position: "x", partName: state.self.partName})]})
  .finish()
export const s2 = forklift.designEmpty('s2').finish()

s0.react([Events.partReq], s1, (_, e) => {
    if (getRandomInt(0, 10) >= 9) { return { partName: "broken part" } }
    return s1.make({partName: e.payload.partName}) })
s1.react([Events.pos], s0, (_, e) => { return s0.make() })
s0.react([Events.closingTime], s2, (_, e) => { return s2.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(warehouseProtocol, subsWarehouse, "FL", forklift.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapted machine
const [forkliftAdapted, s0Adapted] = Composition.adaptMachine('FL', warehouseFactoryProtocol, subsComposition, 0, [forklift, s0], true).data!

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('warehouse-factory')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, forkliftAdapted)
  printState(forkliftAdapted.machineName, s0Adapted.mechanism.name, undefined)

  for await (const state of machine) {
    if(state.isLike(s1)) {
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