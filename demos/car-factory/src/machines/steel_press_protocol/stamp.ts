import { Events, Composition, SteelPressProtocol } from './../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

// Using the machine runner DSL an implmentation of stamp in the steel press protocol:
export const stamp = Composition.makeMachine(SteelPressProtocol.stampRole)
export const s0 = stamp.designEmpty('s0').finish()
export const s1 = stamp.designEmpty('s1')
  .command(SteelPressProtocol.cmdPressSteel, [Events.steelParts], (_, part: string) => {
    return [Events.steelParts.make({part: part})]
  }).finish()
export const s2 = stamp.designEmpty('s2').finish()

s0.react([Events.steelRoll], s1, (_) => { return s1.make() })
s1.react([Events.steelParts], s0, (_) => { return s0.make() })
s0.react([Events.carBody], s2, (_) => { return s2.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([SteelPressProtocol.protocol], SteelPressProtocol.subscriptions, SteelPressProtocol.stampRole, stamp.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))