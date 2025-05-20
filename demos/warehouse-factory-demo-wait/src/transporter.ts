import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, printState, projectionInfoTransport, warehouse_factory_protocol, subs_composition, warehouse_protocol, subs_warehouse } from './protocol'
import * as readline from 'readline';
import chalk from "chalk";
import { checkComposedProjection } from '@actyx/machine-check';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

const parts = ['tire', 'windshield', 'chassis', 'hood', 'spoiler']

// Using the machine runner DSL an implmentation of transporter in warehouse w.r.t. subs_warehouse is:
const transporter = Composition.makeMachine('Transport')
export const s0 = transporter.designEmpty('s0')
  .command('request', [Events.partID], (ctx) => {
    var id = parts[Math.floor(Math.random() * parts.length)];
    return [Events.partID.make({ partName: id })]
  })
  .finish()
export const s1 = transporter.designEmpty('s1').finish()
export const s2 = transporter.designState('s2').withPayload<{ partName: string }>()
  .command('deliver', [Events.part], (ctx) => {
    return [Events.part.make({ partName: ctx.self.partName })]
  })
  .finish()
export const s3 = transporter.designEmpty('s3').finish()

// Add reactions
s0.react([Events.partID], s1, (_, e) => { return s1.make() })
s0.react([Events.time], s3, (_, e) => { return s3.make() })
s1.react([Events.pos], s2, (_, e) => {
  return s2.make({ partName: e.payload.partName })
})
s2.react([Events.part], s0, (_, e) => { return s0.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(warehouse_protocol, subs_warehouse, "T", transporter.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapted machine
//const [transportAdapted, s0Adapted] = Composition.adaptMachine('Transport', projectionInfoTransport, Events.allEvents, s0, true)
//const [transportAdapted, s0Adapted] = Composition.adaptMachine('Transport', 'T', warehouse_factory_protocol, subs_composition, s0, true).data!
const [transportAdapted, s0Adapted] = Composition.adaptMachine('Transport', 'T', warehouse_factory_protocol, subs_composition, 0, transporter.createJSONForAnalysis(s0), s0, true).data!

// Run the machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('warehouse-factory')
  const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, projectionInfoTransport)
  printState(transportAdapted.machineName, s0Adapted.mechanism.name, undefined)
  console.log(chalk.bgBlack.red.dim`    partID!`);

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