import { Events, Protocol, subsWarehouseFactoryQuality, quality, subsQuality, observe, throwMachineImplementationErrors, factory, warehouse, testCar  } from '../protocol'
import { checkComposedProjection } from '@actyx/machine-check'
import { QualityControl } from '../protocol'

// QualityControl machine using the Actyx machine-runner library
const qualityControl = Protocol.makeMachine(QualityControl)
export const initialState = qualityControl.designEmpty("initialState")
    .command(observe, [Events.observingEvent], (_) => [Events.observingEvent.make({})])
    .finish()
export const observingState = qualityControl.designEmpty("observingState").finish()
export const reportingState = qualityControl.designState("reportingState").withPayload<{modelName: string, decision: string}>()
    .command(testCar, [Events.reportEvent], (ctx) =>
        [Events.reportEvent.make({modelName: ctx.self.modelName, decision: ctx.self.decision})])
    .finish()
export const finalState = qualityControl.designEmpty("finalState").finish()

initialState.react([Events.observingEvent], observingState, () => observingState.make() )
observingState.react([Events.carEvent], reportingState, (_, event) =>
    event.payload.partName !== "broken part"
    ? reportingState.make({modelName: event.payload.modelName, decision: "ok"})
    : reportingState.make({ modelName: event.payload.modelName, decision: "notOk"}))

reportingState.react([Events.reportEvent], finalState, () => finalState.make())

// Check that the machine is correctly implemented w.r.t. the quality control protocol
const checkMachineResult = checkComposedProjection([quality], subsQuality, QualityControl, qualityControl.createJSONForAnalysis(initialState))
if (checkMachineResult.type === "ERROR") {
    throwMachineImplementationErrors(checkMachineResult)
}

// Adapted machine
export const [qualityWarehouseFactoryQuality, initialStateWarehouseFactoryQuality] = Protocol.adaptMachine(
    QualityControl,
    [warehouse, factory, quality],
    2,
    subsWarehouseFactoryQuality,
    [qualityControl, initialState],
    true
).data!
