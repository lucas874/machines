import { checkComposedProjection } from "@actyx/machine-check";
import { closeDoor, Door, Events, factory, Protocol, subsWarehouseFactory, subsWarehouse, throwMachineImplementationErrors, warehouse, quality, subsWarehouseFactoryQuality } from "../protocol";

// Door machine implementation using the Actyx toolkit
export const door = Protocol.makeMachine(Door)

// States
export const initialState = door.designEmpty("initialState")
    .command(closeDoor, [Events.closingTimeEvent], () =>
        [Events.closingTimeEvent.make({ timeOfDay: new Date().toLocaleTimeString() })])
    .finish()
export const requestedState = door.designEmpty("requestedState").finish()
export const closedState = door.designEmpty("closedState").finish()

// Accept events and change states
initialState.react([Events.partReqEvent], requestedState, () => requestedState.make())
requestedState.react([Events.partOKEvent], initialState, () => initialState.make())
initialState.react([Events.closingTimeEvent], closedState, () => closedState.make())

// Check that the machine is a correct implementation w.r.t. the warehouse protocol.
const checkMachineResult = checkComposedProjection(
    [warehouse],    // Swarm protocol to check machine against.
    subsWarehouse,  // Subscriptions for the (generated in src/protocol.ts)
    Door,           // Role played by the machine
    door.createJSONForAnalysis(initialState) // Exctracted structure of machine implementation.
)
if (checkMachineResult.type === "ERROR") {
    throwMachineImplementationErrors(checkMachineResult)
}

// Adapted machine for warehouse || factory
export const [doorWarehouseFactory, initialStateWarehouseFactory] = Protocol.adaptMachine(
    Door,                 // Role played by the machine to adapt.
    [warehouse, factory], // Swarm protocols in the composition.
    0,                    // Index of the warehouse protocol in the array above.
    subsWarehouseFactory, // Subscriptions for the composition (generated in src/protocol.ts)
    [door, initialState], // Original door machine implementation and its initial state.
    true                  // 'Verbose' option: print machine state and transitions.
).data!                   // Unwrapping the result returned by adaptMachine().

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