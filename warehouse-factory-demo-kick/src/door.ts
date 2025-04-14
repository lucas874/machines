import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT} from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, getRandomInt } from './protocol'
import { checkComposedProjection, projectionAndInformation } from '@actyx/machine-check'

const doorFinal = "{ { { 3 } } || { { 0 } }, { { 3 } } || { { 2 } } }"

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

// Projection of Gwarehouse || Gfactory || Gquality over D
const projectionInfoResult = projectionAndInformation(interfacing_swarms, subs, "D")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

// Adapted machine
const [doorAdapted, s0_] = Composition.adaptMachine("D", projectionInfo, Events.allEvents, s0)
const checkProjResult = checkComposedProjection(interfacing_swarms, subs, "D", doorAdapted.createJSONForAnalysis(s0_))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const machine = createMachineRunnerBT(app, tags, s0_, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)

    for await (const state of machine) {
      console.log("door. state is:", state.type)
      if (state.payload !== undefined) {
        console.log("state payload is:", state.payload)
      }
      console.log()
      if (state.type === doorFinal) {
        console.log("\x1b[32mDoor reached its final state. Press CTRL + C to exit.\x1b[0m")
      }
      const s = state.cast()
      for (var c in s.commands()) {
          if (c === 'close') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1 || {}).includes('close')) {
                    s1.close()
                }
            }, getRandomInt(5500, 8000))
            break
          }
      }
    }
    app.dispose()
}

main()