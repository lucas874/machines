import { Actyx } from '@actyx/sdk'
import { createMachineRunner, createMachineRunnerBT } from '@actyx/machine-runner'
import { Events, manifest, Composition, getRandomInt, warehouse_protocol, subs_warehouse, print_event, warehouse_factory_quality_protocol, subs_composition } from './protocol'
import { checkComposedProjection, ResultData, ProjectionAndSucceedingMap, projectionAndInformation, projectionAndInformationNew } from '@actyx/machine-check'

const parts = ['tire', 'windshield', 'chassis', 'hood', 'spoiler']

// Using the machine runner DSL an implmentation of transporter in warehouse w.r.t. subs_warehouse is:
const transporter = Composition.makeMachine('T')
export const s0 = transporter.designEmpty('s0')
    .command('request', [Events.partReq], (s: any) => {
      var id = parts[Math.floor(Math.random() * parts.length)];
      console.log("requesting a", id);
      return [Events.partReq.make({id: id})]})
    .finish()
export const s1 = transporter.designEmpty('s1').finish()
export const s2 = transporter.designState('s2').withPayload<{part: string}>()
    .command('deliver', [Events.partOK], (s: any, e: any) => {
      console.log("delivering a", s.self.part)
      return [Events.partOK.make({part: s.self.part})] })
    .finish()
export const s3 = transporter.designEmpty('s3').finish()

s0.react([Events.partReq], s1, (_, e) => { print_event(e); return s1.make() })
s0.react([Events.closingTime], s3, (_, e) => { print_event(e); return s3.make() })
s1.react([Events.pos], s2, (_, e) => {
    print_event(e)
    console.log("got a ", e.payload.part);
    return { part: e.payload.part } })

s2.react([Events.partOK], s0, (_, e) => { print_event(e); return s0.make() })

// Check that the original machine is a correct implementation. A prerequisite for reusing it.
const checkProjResult = checkComposedProjection(warehouse_protocol, subs_warehouse, "T", transporter.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Projection of warehouse || factory || quality over T
const projectionInfoResult = projectionAndInformationNew(warehouse_factory_quality_protocol, subs_composition, "T", transporter.createJSONForAnalysis(s0), 0)
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data
//console.log(JSON.stringify(projectionInfo1, null, 2))

// Adapted machine
const [transporterAdapted, s0_] = Composition.adaptMachineNew("T", projectionInfo, Events.allEvents, s0)

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-factory-quality')
    const machine = createMachineRunnerBT(app, tags, s0_, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)
    //const machine = createMachineRunner(app, tags, s0, undefined)
    for await (const state of machine) {
      console.log("Transporter. State is:", state.type)
      if (state.payload !== undefined) {
        console.log("State payload is:", state.payload)
      }
      console.log(state.isLike(s0))
      console.log()
      const s = state.cast()
      //if(state.hasCommand('request')) {
      if(state.isLike(s0)) {
        console.log("boing")
        state.cast().commands()?.request()
      }

      for (var c in s.commands()) {
          if (c === 'request') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1 || {}).includes('request')) {
                    s1.request()
                }
            }, getRandomInt(2000, 5000))
            break
          }
          if (c === 'deliver') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1 || {}).includes('deliver')) {
                    s1.deliver()
                }
            }, getRandomInt(4000, 8000))
            break
          }
      }
    }
    app.dispose()
}

main()