import { Actyx } from '@actyx/sdk'
import { createMachineRunner } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, getRandomInt } from './warehouse_protocol'
import { checkComposedProjection } from '@actyx/machine-check'

// Using the machine runner DSL an implmentation of forklift in Gwarehouse is:
const forklift = Composition.makeMachine('FL')
export const s0 = forklift.designEmpty('s0') .finish()
export const s1 = forklift.designState('s1').withPayload<{id: string}>()
  .command('get', [Events.pos], (state: any, _: any) => {
    console.log("retrieved a", state.self.id, "at position x");
    return [Events.pos.make({position: "x", part: state.self.id})]})
  .finish()
export const s2 = forklift.designEmpty('s2').finish()

s0.react([Events.partReq], s1, (_, e) => {
    console.log("a", e.payload.id, "was requested");
    if (getRandomInt(0, 10) >= 9) { return { id: "broken part" } }
    return s1.make({id: e.payload.id}) })
s1.react([Events.pos], s0, (_) => s0.make())
s0.react([Events.closingTime], s2, (_) => s2.make())

const checkProjResult = checkComposedProjection(interfacing_swarms, subs, "FL", forklift.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-1')
    const machine = createMachineRunner(app, tags, s0, undefined)

    for await (const state of machine) {
      console.log("forklift. state is:", state.type)
      if (state.payload !== undefined) {
        console.log("state payload is:", state.payload)
      }
      console.log()
      const s = state.cast()
      for (var c in s.commands()) {
          if (c === 'get') {
            setTimeout(() => {
              var s1 = machine.get()?.cast()?.commands() as any
              if (Object.keys(s1 || {}).includes('get')) {
                s1.get()
              }
            }, 1500)
            break
          }
      }
    }
    app.dispose()
}

main()