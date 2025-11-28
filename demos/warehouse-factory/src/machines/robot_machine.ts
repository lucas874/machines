import { checkComposedProjection } from "@actyx/machine-check";
import { buildCar, Events, factory, PartPayload, Protocol, Robot, subsWarehouseFactory, subsFactory, throwMachineImplementationErrors, warehouse, quality, subsWarehouseFactoryQuality } from "../protocol";

// Factory robot machine implemented using Actyx machine-runner
export const robot = Protocol.makeMachine(Robot)

export const initialState = robot.designEmpty("initialState").finish()

export const buildState = robot.designState("buildState").withPayload<PartPayload>()
  .command(buildCar, [Events.carEvent], (ctx) => {
    const modelName = ctx.self.partName === 'spoiler' ? "sports car" : "sedan";
    return [Events.carEvent.make({partName: ctx.self.partName, modelName: modelName})]})
  .finish()

export const carFinishedState = robot.designEmpty("carFinishedState").finish()

export const carCheckedState = robot.designEmpty("carCheckedState").finish()

// Add reactions
initialState.react([Events.partOKEvent], buildState, (_, event) => buildState.make({partName: event.payload.partName}))
buildState.react([Events.carEvent], carFinishedState, () => carFinishedState.make())

// Check that the machine is a correct implementation w.r.t. the factory protocol.
const checkMachineResult = checkComposedProjection([factory], subsFactory, Robot, robot.createJSONForAnalysis(initialState))
if (checkMachineResult.type === "ERROR") {
    throwMachineImplementationErrors(checkMachineResult)
}

// Adapted machine for warehouse || factory
export const [robotWarehouseFactory, initialStateWarehouseFactory] = Protocol.adaptMachine(
    Robot,
    [warehouse, factory],
    1,
    subsWarehouseFactory,
    [robot, initialState],
    true
).data!

// Adapted machine for warehouse || factory || quality
export const [robotWarehouseFactoryQuality, initialStateWarehouseFactoryQuality] = Protocol.adaptMachine(
    Robot,
    [warehouse, factory, quality],
    1,
    subsWarehouseFactoryQuality,
    [robot, initialState],
    true
).data!