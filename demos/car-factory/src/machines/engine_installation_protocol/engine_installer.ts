import { Events, Composition, EngineInstallationProtocol, type EngineInstallationPayload } from './../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

type RequestEnginePayload = { shape: string, color: string }

// Using the machine runner DSL an implmentation of the engine installer in the engine installation protocol:
export const engineInstaller = Composition.makeMachine(EngineInstallationProtocol.engineInstallerRole)
export const s0 = engineInstaller.designEmpty('s0').finish()
export const s1 = engineInstaller.designState('s1')
    .withPayload<RequestEnginePayload>()
    .command(EngineInstallationProtocol.cmdRequestEngine, [Events.requestEngine], (ctx) => {
        const engine = ctx.self.shape === "truck" ? "truckEngine" : "basicEngine"
        return [Events.requestEngine.make({ item: engine, to: "myPosition" })]
    })
    .finish()
export const s2 = engineInstaller.designState('s2')
    .withPayload<EngineInstallationPayload>()
    .finish()
export const s3 = engineInstaller.designState('s3')
    .withPayload<EngineInstallationPayload>()
    .command(EngineInstallationProtocol.cmdInstallEngine, [Events.engineInstalled], (ctx) =>
        [Events.engineInstalled.make(ctx.self)])
    .finish()
export const s4 = engineInstaller.designEmpty('s4').finish()


s0.react([Events.paintedCarBody], s1, (_, event) => { return s1.make({ shape: event.payload.shape, color: event.payload.color }) })
s1.react([Events.requestEngine], s2, (ctx, event) => { return s2.make({ shape: ctx.self.shape, color: ctx.self.color, engine: event.payload.item }) })
s2.react([Events.itemDelivery], s3, (ctx) => { return s3.make(ctx.self) })
s3.react([Events.engineInstalled], s4, (_) => { return s4.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([EngineInstallationProtocol.protocol], EngineInstallationProtocol.subscriptions, EngineInstallationProtocol.engineInstallerRole, engineInstaller.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))