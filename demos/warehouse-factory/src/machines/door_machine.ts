import { checkComposedProjection } from "@actyx/machine-check";
import { closeDoor, Door, Events, factory, Protocol, subsWarehouseFactory, subsWarehouse, throwMachineImplementationErrors, warehouse, quality, subsWarehouseFactoryQuality } from "../protocol";

// Door machine using the Actyx machine-runner library
export const door = Protocol.makeMachine(Door)

export const initialState = door.designEmpty("initialState")
    .command(closeDoor, [Events.closingTimeEvent], () =>
        [Events.closingTimeEvent.make({ timeOfDay: new Date().toLocaleTimeString() })])
    .finish()
export const requestedState = door.designEmpty("requestedState").finish()
export const closedState = door.designEmpty("closedState").finish()

// Add reactions
initialState.react([Events.partReqEvent], requestedState, () => requestedState.make())
requestedState.react([Events.partOKEvent], initialState, () => initialState.make())
initialState.react([Events.closingTimeEvent], closedState, () => closedState.make())

// Check that the machine is a correct implementation w.r.t. the warehouse protocol.
const checkMachineResult = checkComposedProjection([warehouse], subsWarehouse, Door, door.createJSONForAnalysis(initialState))
if (checkMachineResult.type === "ERROR") {
    throwMachineImplementationErrors(checkMachineResult)
}

// Adapted machine for warehouse || factory
export const [doorWarehouseFactory, initialStateWarehouseFactory] = Protocol.adaptMachine(
    Door,                   // The role played by the machine to adapt.
    [warehouse, factory],   // The swarm protocols in the composition.
    0,                      // The index of the warehouse protocol in the array above.
    subsWarehouseFactory,   // The subscriptions for the composition. Automatically generated in src/protocol.ts.
    [door, initialState],   // The original door implementation and its initial state.
    true                    // Optional parameter to make the machine 'verbose', printing its current state and transitions.
).data!                     // Unwrapping the result returned by adaptMachine().

// Original but branch tracking machine
export const [doorBT, initialStateBT] = Protocol.adaptMachine(
    Door,
    [warehouse],
    0,
    subsWarehouse,
    [door, initialState],
    true
).data!

// Adapted machine for warehouse || factory || quality
export const [doorWarehouseFactoryQuality, initialStateWarehouseFactoryQuality] = Protocol.adaptMachine(
    Door,
    [warehouse, factory, quality],
    0,
    subsWarehouseFactoryQuality,
    [door, initialState],
    true
).data!