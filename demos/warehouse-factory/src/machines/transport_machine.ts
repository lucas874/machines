import { checkComposedProjection } from "@actyx/machine-check";
import { deliver, Events, factory, PartPayload, Protocol, request, subsWarehouseFactory, subsWarehouse, throwMachineImplementationErrors, Transport, warehouse, quality, subsWarehouseFactoryQuality } from "../protocol";

const parts = [ "tire", "windshield", "chassis", "hood", "spoiler" ]
const partToRequest = (): string => parts[Math.floor(Math.random() * parts.length)]

// Implement the transport machine using the Actyx machine-runner library
export const transport = Protocol.makeMachine(Transport)

export const initialState = transport.designEmpty("initialState")
    .command(request, [Events.partReqEvent], (_) => {
        return [Events.partReqEvent.make({ partName: partToRequest() })]
    })
    .finish()

export const requestedState = transport.designEmpty("requestedState").finish()

export const deliverState = transport.designState("deliverState")
    .withPayload<PartPayload>()
    .command(deliver, [Events.partOKEvent], (ctx) => {
        return [Events.partOKEvent.make({ partName: ctx.self.partName })]
    })
    .finish()

export const closedState = transport.designEmpty("closedState").finish()

// Add reactions
initialState.react([Events.partReqEvent], requestedState, () => requestedState.make())
initialState.react([Events.closingTimeEvent], closedState, () => closedState.make())
requestedState.react([Events.positionEvent], deliverState, (_, event) =>
    deliverState.make( {partName: event.payload.position }))
deliverState.react([Events.partOKEvent], initialState, () => initialState.make())

// Check that the machine is implemented correctlty w.r.t. the warehouse protocol
const checkMachineResult = checkComposedProjection([warehouse], subsWarehouse, Transport, transport.createJSONForAnalysis(initialState))
if (checkMachineResult.type === "ERROR") {
    throwMachineImplementationErrors(checkMachineResult)
}

// Adapted machine
export const [transportWarehouseFactory, initialStateWarehouseFactory] = Protocol.adaptMachine(
    Transport,
    [warehouse, factory],
    0,
    subsWarehouseFactory,
    [transport, initialState],
    true
).data!

// Original but branch tracking machine
export const [transportBT, initialStateBT] = Protocol.adaptMachine(
    Transport,
    [warehouse],
    0,
    subsWarehouse,
    [transport, initialState],
    true
).data!

// Adapted machine for warehouse || factory || quality
export const [transportWarehouseFactoryQuality, initialStateWarehouseFactoryQuality] = Protocol.adaptMachine(
    Transport,
    [warehouse, factory, quality],
    0,
    subsWarehouseFactoryQuality,
    [transport, initialState],
    true
).data!