import { Events, Composition, EngineInstallationProtocol, type EngineInstallationPayload } from '../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

export const engineChecker = Composition.makeMachine(EngineInstallationProtocol.engineCheckerRole)
export const s0 = engineChecker.designEmpty('s0').finish()
export const s1 = engineChecker.designState('s1')
    .withPayload<EngineInstallationPayload>()
    .command(EngineInstallationProtocol.cmdCheckEngine, [Events.engineChecked], (ctx) => {
        return [Events.engineChecked.make(ctx.self)]
    })
    .finish()
export const s2 = engineChecker.designEmpty('s2').finish()

s0.react([Events.engineInstalled], s1, (_, event) => { 
    const {shape, color, engine} = event.payload;
    return s1.make({shape, color, engine})})
s1.react([Events.engineChecked], s2, () => { return s2.make()})

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([EngineInstallationProtocol.protocol], EngineInstallationProtocol.subscriptions, EngineInstallationProtocol.engineCheckerRole, engineChecker.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))