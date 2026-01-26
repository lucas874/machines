import { Events, TransportOrder, subsWarehouse, transportOrderProtocol, assemblyLineProtocol, subscriptions } from './protocol'
import { checkComposedProjection } from '@actyx/machine-check';

// initialize the state machine builder for the `warehouse` role
export const Warehouse =
  TransportOrder.makeMachine('warehouse')

// add initial state with command to request the transport
export const InitialWarehouse = Warehouse
  .designEmpty('Initial')
  .command('request', [Events.request], (_ctx, id: string, from: string, to: string) => [{ id, from, to }])
  .finish()

// add state entered after performing the request
export const AuctionWarehouse = Warehouse
  .designEmpty('AuctionWarehouse')
  .finish()

// add state entered after a transport robot has been selected
export const SelectedWarehouse = Warehouse
  .designEmpty('SelectedWarehouse')
  .finish()

// add state for acknowledging a delivery entered after a robot has performed the delivery
export const AcknowledgeWarehouse = Warehouse
  .designState('Acknowledge')
  .withPayload<{id: string}>()
  .command('acknowledge', [Events.ack], (ctx) => [{ id: ctx.self.id }])
  .finish()

export const DoneWarehouse = Warehouse.designEmpty('Done').finish()

// describe the transition into the `AuctionWarehouse` state after request has been made
InitialWarehouse.react([Events.request], AuctionWarehouse, (_ctx, _event) => {})
// describe the transitions from the `AuctionWarehouse` state
AuctionWarehouse.react([Events.bid], AuctionWarehouse, (_ctx, _event) => {})
AuctionWarehouse.react([Events.selected], SelectedWarehouse, (_ctx, _event) => {})
// describe the transitions from the `SelectedWarehouse` state
SelectedWarehouse.react([Events.deliver], AcknowledgeWarehouse, (_ctx, event) => AcknowledgeWarehouse.make({id: event.payload.id}))
// describe the transitions from the `AcknoweledgeWarehouse` state
AcknowledgeWarehouse.react([Events.ack], DoneWarehouse, (_ctx, _event) => {})

// Adapted machine.
export const [warehouseAdapted, initialWarehouseAdapted] = TransportOrder.adaptMachine('warehouse', [transportOrderProtocol, assemblyLineProtocol], 0, subscriptions, [Warehouse, InitialWarehouse], true).data!
































