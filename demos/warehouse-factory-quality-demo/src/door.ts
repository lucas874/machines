import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT} from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_quality_protocol, subs_composition, getRandomInt, warehouse_protocol, subs_warehouse, print_event } from './protocol'
import { checkComposedProjection } from '@actyx/machine-check'

// Using the machine runner DSL an implmentation of door in warehouse w.r.t. subs_warehouse is:
const door = Composition.makeMachine('D')
export const s0 = door.designEmpty('s0')
    .command('close', [Events.closingTime], () => {
        var dateString = new Date().toLocaleString();
        console.log("closed warehouse at:", dateString);
        return [Events.closingTime.make({timeOfDay: dateString})]})
    .finish()
export const s1 = door.designEmpty('s1').finish()
export const s2 = door.designEmpty('s2').finish()

s0.react([Events.partReq], s1, (_, e) => { print_event(e); return s1.make() })
s1.react([Events.partOK], s0, (_, e) => { print_event(e); return s0.make() })
s0.react([Events.closingTime], s2, (_, e) => { print_event(e); return s2.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(warehouse_protocol, subs_warehouse, "D", door.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", \n"))

// Adapt machine
const [doorAdapted, s0Adapted] = Composition.adaptMachine('D', warehouse_factory_quality_protocol, 0, subs_composition, [door, s0]).data!

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-factory-quality')
    const machine = createMachineRunnerBT(app, tags, s0Adapted, undefined, doorAdapted)

    for await (const state of machine) {
      console.log("Door. State is:", state.type)
      if (state.payload !== undefined) {
        console.log("State payload is:", state.payload)
      }
      console.log()
      if (state.isLike(s0)) {
        setTimeout(() => {
          const stateAfterTimeOut = machine.get()
          if (stateAfterTimeOut?.isLike(s0)) {
            stateAfterTimeOut?.cast().commands()?.close()
          }
        }, getRandomInt(4000, 8000))
      }
    }
    app.dispose()
}

main()