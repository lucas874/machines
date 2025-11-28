import { checkComposedProjection } from "@actyx/machine-check";
import { Events, factory, Forklift, get, getRandomInt, PartPayload, Protocol, subsWarehouseFactory, subsWarehouse, throwMachineImplementationErrors, warehouse, quality, subsWarehouseFactoryQuality } from "../protocol";

// Forklift machine using the Actyx machine-runner library
export const forklift = Protocol.makeMachine(Forklift)
export const initialState = forklift.designEmpty("initialState").finish()
export const deliverState = forklift.designState("deliverState")
    .withPayload<PartPayload>()
    .command(get, [Events.positionEvent], (ctx) =>
        [Events.positionEvent.make( {position: "x", partName: ctx.self.partName })])
    .finish()
export const closedState = forklift.designEmpty("closedState").finish()

// Add reactions
initialState.react([Events.partReqEvent], deliverState, (_, event) =>
    getRandomInt(0, 10) >= 9 ? deliverState.make({ partName: "broken part" }) : deliverState.make({partName: event.payload.partName}))
deliverState.react([Events.positionEvent], initialState, () => initialState.make())
initialState.react([Events.closingTimeEvent], closedState, () => closedState.make())

// Check that the machine is correctly implemented w.r.t. the warehouse protocol
const checkMachineResult = checkComposedProjection([warehouse], subsWarehouse, Forklift, forklift.createJSONForAnalysis(initialState))
if (checkMachineResult.type === "ERROR") {
    throwMachineImplementationErrors(checkMachineResult)
}

// Adapted machine for warehouse || factory
export const [forkliftWarehouseFactory, initialStateWarehouseFactory] = Protocol.adaptMachine(
    Forklift,
    [warehouse, factory],
    0,
    subsWarehouseFactory,
    [forklift, initialState],
    true
).data!

// Original but branch tracking machine
export const [forkliftBT, initialStateBT] = Protocol.adaptMachine(
    Forklift,
    [warehouse],
    0,
    subsWarehouse,
    [forklift, initialState],
    true
).data!

// Adapted machine for warehouse || factory || quality
export const [forkliftWarehouseFactoryQuality, initialStateWarehouseFactoryQuality] = Protocol.adaptMachine(
    Forklift,
    [warehouse, factory, quality],
    0,
    subsWarehouseFactoryQuality,
    [forklift, initialState],
    true
).data!