import { Events, Composition, WindowInstallationProtocol, type WindowInstallationPayload } from '../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

export const windowChecker = Composition.makeMachine(WindowInstallationProtocol.windowCheckerRole)
export const s0 = windowChecker.designEmpty('s0').finish()
export const s1 = windowChecker.designState('s1')
    .withPayload<WindowInstallationPayload>()
    .command(WindowInstallationProtocol.cmdWindowsDone, [Events.windowsDone], (ctx) => {
            return [Events.windowsDone.make(ctx.self)]
        })
    .finish()
export const s2 = windowChecker.designState('s2')
    .withPayload<WindowInstallationPayload>()
    .finish()
export const s3 = windowChecker.designEmpty('s3').finish()

s0.react([Events.engineChecked], s1, (_, event) => {
    const {shape, color, engine} = event.payload;
    return s1.make({shape, color, engine, numWindows: 0})
})
s1.react([Events.windowPickup], s2, (ctx) => { return s2.make(ctx.self)})
s2.react([Events.windowInstalled], s1, (_, event) => {
    const {shape, color, engine, numWindows} = event.payload;
    return s1.make({shape, color, engine, numWindows})})
s1.react([Events.windowsDone], s3, () => s3.make())

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([WindowInstallationProtocol.protocol], WindowInstallationProtocol.subscriptions, WindowInstallationProtocol.windowCheckerRole, windowChecker.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))