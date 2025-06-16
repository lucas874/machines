import { Actyx } from '@actyx/sdk'
import { createMachineRunner, createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_protocol, subs_warehouse, warehouse_factory_quality_protocol, subs_composition, printState } from './protocol'
import { checkComposedProjection } from '@actyx/machine-check'
import chalk from 'chalk'
import * as readline from 'readline';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

const parts = ['tire', 'windshield', 'chassis', 'hood', 'spoiler']

// Using the machine runner DSL an implmentation of transporter in warehouse w.r.t. subs_warehouse is:
const transporter = Composition.makeMachine('T')
export const s0 = transporter.designEmpty('s0')
    .command('request', [Events.partReq], (s: any) => {
      var id = parts[Math.floor(Math.random() * parts.length)];
      return [Events.partReq.make({id: id})]})
    .finish()
export const s1 = transporter.designEmpty('s1').finish()
export const s2 = transporter.designState('s2').withPayload<{part: string}>()
    .command('deliver', [Events.partOK], (s: any) => {
      return [Events.partOK.make({part: s.self.part})] })
    .finish()
export const s3 = transporter.designEmpty('s3').finish()

s0.react([Events.partReq], s1, (_, e) => { return s1.make() })
s0.react([Events.closingTime], s3, (_, e) => { return s3.make() })
s1.react([Events.pos], s2, (_, e) => {
    return { part: e.payload.part } })

s2.react([Events.partOK], s0, (_, e) => { return s0.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(warehouse_protocol, subs_warehouse, "T", transporter.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Adapted machine
const [transportAdapted, s0Adapted] = Composition.adaptMachine('T', warehouse_factory_quality_protocol, subs_composition, 0, [transporter, s0], true).data!

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-factory-quality')
    const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, transportAdapted)
    printState(transportAdapted.machineName, s0Adapted.mechanism.name, undefined)
    console.log(chalk.bgBlack.red.dim`    partReq!`);

    for await (const state of machine) {
      if(state.isLike(s0)) {
        rl.on('line', (_) => {
          const stateAfterTimeOut = machine.get()
          if (stateAfterTimeOut?.isLike(s0)) {
            stateAfterTimeOut?.cast().commands()?.request()
          }
        })
      }

      if(state.isLike(s2)) {
        rl.on('line', (_) => {
          const stateAfterTimeOut = machine.get()
          if (stateAfterTimeOut?.isLike(s2)) {
            stateAfterTimeOut?.cast().commands()?.deliver()
          }
        })
      }
    }
    app.dispose()
}

main()