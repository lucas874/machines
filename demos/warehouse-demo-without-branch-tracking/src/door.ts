import { Actyx } from '@actyx/sdk'
import { createMachineRunner } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, getRandomInt } from './warehouse_protocol'
import { checkComposedProjection } from '@actyx/machine-check'

// Using the machine runner DSL an implmentation of door in Gwarehouse is:
const door = Composition.makeMachine('D')
export const s0 = door.designEmpty('s0')
    .command('close', [Events.closingTime], () => {
        var dateString = new Date().toLocaleString();
        console.log("closed warehouse at:", dateString);
        return [Events.closingTime.make({timeOfDay: dateString})]})
    .finish()
export const s1 = door.designEmpty('s1').finish()
export const s2 = door.designEmpty('s2').finish()

s0.react([Events.partReq], s1, (_) => s1.make())
s1.react([Events.partOK], s0, (_) => s0.make())
s0.react([Events.closingTime], s2, (_) => s2.make())

const checkProjResult = checkComposedProjection(interfacing_swarms, subs, "D", door.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-1')
    const machine = createMachineRunner(app, tags, s0, undefined)

    for await (const state of machine) {
      console.log("door. state is:", state.type)
      if (state.payload !== undefined) {
        console.log("state payload is:", state.payload)
      }
      console.log()
      const s = state.cast()
      for (var c in s.commands()) {
          if (c === 'close') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1 || {}).includes('close')) {
                    s1.close()
                }
            }, getRandomInt(5000, 8000))
            break
          }
      }
    }
    app.dispose()
}

main()