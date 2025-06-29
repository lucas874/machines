import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, printState, warehouseFactoryProtocols, subsComposition, warehouseProtocol, subsWarehouse } from './protocol'
import * as readline from 'readline';
import chalk from "chalk";
import { checkComposedProjection } from '@actyx/machine-check';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

const parts = ['tire', 'windshield', 'chassis', 'hood', 'spoiler']

// Using the machine runner DSL an implmentation of transporter in warehouse w.r.t. subs_warehouse is:
const transport = Composition.makeMachine('Transport')
export const s0 = transport.designEmpty('s0')
  .command('request', [Events.partReq], (ctx) => {
    var id = parts[Math.floor(Math.random() * parts.length)];
    return [Events.partReq.make({ partName: id })]
  })
  .finish()
export const s1 = transport.designEmpty('s1').finish()
export const s2 = transport.designState('s2').withPayload<{ partName: string }>()
  .command('deliver', [Events.partOK], (ctx) => {
    return [Events.partOK.make({ partName: ctx.self.partName })]
  })
  .finish()
export const s3 = transport.designEmpty('s3').finish()

// Add reactions
s0.react([Events.partReq], s1, (_, e) => { return s1.make() })
s0.react([Events.closingTime], s3, (_, e) => { return s3.make() })
s1.react([Events.pos], s2, (_, e) => {
  return s2.make({ partName: e.payload.partName })
})
s2.react([Events.partOK], s0, (_, e) => { return s0.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(warehouseProtocol, subsWarehouse, "T", transport.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))







































// Adapted machine
const [transportAdapted, s0Adapted] = Composition.adaptMachine('T', warehouseFactoryProtocols, 0, subsComposition, [transport, s0], true).data!























// Run the machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('warehouse-factory')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, transportAdapted)
  printState(transportAdapted.machineName, s0Adapted.mechanism.name, undefined)
  console.log(chalk.bgBlack.red.dim`    ${Events.partReq.type}!`);

  for await (const state of machine) {
    if (state.isLike(s0)) {
      rl.on('line', (_) => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s0)) {
          stateAfterTimeOut?.cast().commands()?.request()
        }
      })
    }

    if (state.isLike(s2)) {
      rl.on('line', (_) => {
        const stateAfterTimeOut = machine.get()
        if (stateAfterTimeOut?.isLike(s2)) {
          stateAfterTimeOut?.cast().commands()?.deliver()
        }
      })
    }
  }
  rl.close();
  app.dispose()
}

main()