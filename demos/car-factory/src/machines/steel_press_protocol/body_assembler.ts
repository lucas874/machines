import { Events, Composition, SteelPressProtocol } from './../../protocol.js'
import { checkComposedProjection } from '@actyx/machine-check';

type PartsPayload = { parts: string[] }


// Using the machine runner DSL an implmentation of body assembler in the steel press protocol:
export const bodyAssembler = Composition.makeMachine(SteelPressProtocol.bodyAssemblerRole)
export const s0 = bodyAssembler.designState('s0').withPayload<PartsPayload>().finish()
export const s1 = bodyAssembler.designState('s1').withPayload<PartsPayload>().finish()
export const s2 = bodyAssembler.designState('s2').withPayload<PartsPayload>()
  .command(SteelPressProtocol.cmdAssembleBody, [Events.partialCarBody], (ctx) => {
    return [Events.partialCarBody.make({ parts: ctx.self.parts })]
  }).finish()
export const s3 = bodyAssembler.designEmpty('s3').finish()

s0.react([Events.steelRoll], s1, (ctx) => { return s1.make(ctx.self) })
s1.react([Events.steelParts], s2, (ctx, event) => { ctx.self.parts.push(event.payload.part); return s2.make(ctx.self) })
s2.react([Events.partialCarBody], s0, (_, event) => { return s0.make( {parts: event.payload.parts} ) })
s0.react([Events.carBody], s3, (_) => { return s3.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection([SteelPressProtocol.protocol], SteelPressProtocol.subscriptions, SteelPressProtocol.bodyAssemblerRole, bodyAssembler.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))