import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, printState, subsWarehouse, warehouseProtocol, factoryProtocol, subscriptions } from './protocol'
import * as readline from 'readline';
import chalk from "chalk";
import { checkComposedProjection } from '@actyx/machine-check';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout
});

const parts = ['tire', 'windshield', 'chassis', 'hood', 'spoiler']

// initialize the state machine builder for the `warehouse` role
const TransportOrderForWarehouse =
  Composition.makeMachine('warehouse')

// add initial state with command to request the transport
const InitialWarehouse = TransportOrderForWarehouse
  .designEmpty('Initial')
  .command('request', [Events.request], (_ctx, id: string, from: string, to: string) => [{ id, from, to }])
  .finish()

// add state entered after performing the request
const AuctionWarehouse = TransportOrderForWarehouse
  .designEmpty('AuctionWarehouse')
  .finish()

// add state entered after a transport robot has been selected
const SelectedWarehouse = TransportOrderForWarehouse
  .designEmpty('SelectedWarehouse')
  .finish()

// add state for acknowledging a delivery entered after a robot has performed the delivery
const AcknowledgeWarehouse = TransportOrderForWarehouse
  .designState('Acknowledge')
  .withPayload<{id: string}>()
  .command('acknowledge', [Events.ack], (ctx) => [{ id: ctx.self.id }])
  .finish()

const DoneWarehouse = TransportOrderForWarehouse.designEmpty('Done').finish()

// describe the transition into the `AuctionWarehouse` state after request has been made
InitialWarehouse.react([Events.request], AuctionWarehouse, (_ctx, _event) => [{}])
// describe the transitions from the `AuctionWarehouse` state
AuctionWarehouse.react([Events.bid], AuctionWarehouse, (_ctx, _event) => [{}])
AuctionWarehouse.react([Events.selected], SelectedWarehouse, (_ctx, _event) => [{}])
// describe the transitions from the `SelectedWarehouse` state
SelectedWarehouse.react([Events.deliver], AcknowledgeWarehouse, (_ctx, event) => AcknowledgeWarehouse.make({id: event.payload.id}))
// describe the transitions from the `AcknoweledgeWarehouse` state
AcknowledgeWarehouse.react([Events.ack], DoneWarehouse, (_ctx, _event) => [{}])

// Check that the machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([warehouseProtocol], subsWarehouse, "W", TransportOrderForWarehouse.createJSONForAnalysis(InitialWarehouse))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
const [warehouseAdapted, warehouseInitialAdapted] = Composition.adaptMachine('W', [warehouseProtocol, factoryProtocol], 0, subscriptions, [TransportOrderForWarehouse, InitialWarehouse], true).data!

// Run the adapted machine
async function main() {
  const app = await Actyx.of(manifest)
  const tags = Composition.tagWithEntityId('warehouse-factory')
  const machine = createMachineRunnerBT(app, tags, warehouseInitialAdapted, undefined, warehouseAdapted)
  printState(warehouseAdapted.machineName, warehouseInitialAdapted.mechanism.name, undefined)
  console.log(chalk.bgBlack.red.dim`    request!`);

  for await (const state of machine) {
    if (state.isLike(InitialWarehouse)) {
      await state.cast().commands()?.request(parts[Math.floor(Math.random() * parts.length)], "a", "b")
    }
    if (state.isLike(AcknowledgeWarehouse)) {
      await state.cast().commands()?.acknowledge()
    }
  }
  rl.close();
  app.dispose()
}

main()
































