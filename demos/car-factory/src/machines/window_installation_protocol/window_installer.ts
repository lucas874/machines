import { Events, Composition, WindowInstallationProtocol, type WindowInstallationPayload } from './../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

export const windowInstaller = Composition.makeMachine(WindowInstallationProtocol.windowInstallerRole)
export const s0 = windowInstaller.designEmpty('s0').finish()
export const s1 = windowInstaller.designState('s1')
    .withPayload<WindowInstallationPayload>()
    .command(WindowInstallationProtocol.cmdPickUpWindow, [Events.windowPickup], (ctx) => {
            return [Events.windowPickup.make(ctx.self)]
        })
    .finish()
export const s2 = windowInstaller.designState('s2')
    .withPayload<WindowInstallationPayload>()
    .command(WindowInstallationProtocol.cmdInstallwindow, [Events.windowInstalled], (ctx) => {
            return [Events.windowInstalled.make({...ctx.self, numWindows: ctx.self.numWindows + 1})]
        })
    .finish()
export const s3 = windowInstaller.designEmpty('s3').finish()

s0.react([Events.engineChecked], s1, (_, event) => { return s1.make(
    { shape: event.payload.shape, color: event.payload.color, engine: event.payload.engine, numWindows: 0}
)})
s1.react([Events.windowPickup], s2, (_, event) => {
    const {shape, color, engine, numWindows} = event.payload;
    return s2.make({shape, color, engine, numWindows})})
s2.react([Events.windowInstalled], s1, (_, event) => {
    const {shape, color, engine, numWindows} = event.payload;
    return s1.make({shape, color, engine, numWindows})})
s1.react([Events.windowsDone], s3, () => { return s3.make()})

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([WindowInstallationProtocol.protocol], WindowInstallationProtocol.subscriptions, WindowInstallationProtocol.windowInstallerRole, windowInstaller.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))