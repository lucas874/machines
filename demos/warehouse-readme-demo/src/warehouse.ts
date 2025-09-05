import { Events, Composition, subsWarehouse, transportOrderProtocol, assemblyLineProtocol, subscriptions } from './protocol'
import { checkComposedProjection } from '@actyx/machine-check';

// initialize the state machine builder for the `warehouse` role
export const TransportOrderForWarehouse =
  Composition.makeMachine('warehouse')

// add initial state with command to request the transport
export const InitialWarehouse = TransportOrderForWarehouse
  .designEmpty('Initial')
  .command('request', [Events.request], (_ctx, id: string, from: string, to: string) => [{ id, from, to }])
  .finish()

// add state entered after performing the request
export const AuctionWarehouse = TransportOrderForWarehouse
  .designEmpty('AuctionWarehouse')
  .finish()

// add state entered after a transport robot has been selected
export const SelectedWarehouse = TransportOrderForWarehouse
  .designEmpty('SelectedWarehouse')
  .finish()

// add state for acknowledging a delivery entered after a robot has performed the delivery
export const AcknowledgeWarehouse = TransportOrderForWarehouse
  .designState('Acknowledge')
  .withPayload<{id: string}>()
  .command('acknowledge', [Events.ack], (ctx) => [{ id: ctx.self.id }])
  .finish()

export const DoneWarehouse = TransportOrderForWarehouse.designEmpty('Done').finish()

// describe the transition into the `AuctionWarehouse` state after request has been made
InitialWarehouse.react([Events.request], AuctionWarehouse, (_ctx, _event) => {})
// describe the transitions from the `AuctionWarehouse` state
AuctionWarehouse.react([Events.bid], AuctionWarehouse, (_ctx, _event) => {})
AuctionWarehouse.react([Events.selected], SelectedWarehouse, (_ctx, _event) => {})
// describe the transitions from the `SelectedWarehouse` state
SelectedWarehouse.react([Events.deliver], AcknowledgeWarehouse, (_ctx, event) => AcknowledgeWarehouse.make({id: event.payload.id}))
// describe the transitions from the `AcknoweledgeWarehouse` state
AcknowledgeWarehouse.react([Events.ack], DoneWarehouse, (_ctx, _event) => {})

// Adapted machine. Adapting here has no effect. Except that we can make a verbose machine.
export const [warehouseAdapted, warehouseInitialAdapted] = Composition.adaptMachine('warehouse', [transportOrderProtocol, assemblyLineProtocol], 0, subscriptions, [TransportOrderForWarehouse, InitialWarehouse], true).data!
































