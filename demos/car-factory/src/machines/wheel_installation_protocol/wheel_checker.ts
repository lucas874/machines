import { Events, Composition, WheelInstallationProtocol, type WheelInstallationPayload } from '../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

export const wheelChecker = Composition.makeMachine(WheelInstallationProtocol.wheelCheckerRole)
export const s0 = wheelChecker.designEmpty('s0').finish()
export const s1 = wheelChecker.designState('s1')
    .withPayload<WheelInstallationPayload>()
    .command(WheelInstallationProtocol.cmdWheelsDone, [Events.wheelsDone], (ctx) => {
            return [Events.wheelsDone.make(ctx.self)]
        })
    .finish()
export const s2 = wheelChecker.designState('s2')
    .withPayload<WheelInstallationPayload>()
    .finish()
export const s3 = wheelChecker.designEmpty('s3').finish()

s0.react([Events.engineChecked], s1, (_, event) => {
    const {shape, color, engine} = event.payload;
    return s1.make({shape, color, engine, numWheels: 0})
})
s1.react([Events.wheelPickup], s2, (ctx) => { return s2.make(ctx.self)})
s2.react([Events.wheelInstalled], s1, (_, event) => {
    const {shape, color, engine, numWheels} = event.payload;
    return s1.make({shape, color, engine, numWheels})})
s1.react([Events.wheelsDone], s3, () => s3.make())

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([WheelInstallationProtocol.protocol], WheelInstallationProtocol.subscriptions, WheelInstallationProtocol.wheelCheckerRole, wheelChecker.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))