import { Actyx } from '@actyx/sdk'
import { createMachineRunnerBT, createMachineRunner} from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, getRandomInt  } from './warehouse_protocol'
import { checkComposedProjection, ResultData, ProjectionAndSucceedingMap, projectionAndInformation } from '@actyx/machine-check'

const parts = ['tire', 'windshield', 'chassis', 'hood', 'spoiler']

// Using the machine runner DSL an implmentation of transporter in Gwarehouse is:
const transporter = Composition.makeMachine('T')
export const s0 = transporter.designEmpty('s0')
    .command('request', [Events.partReq], (s: any, e: any) => {
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

s0.react([Events.partReq], s1, (_) => s1.make())
s0.react([Events.closingTime], s3, (_) => s3.make())
s1.react([Events.pos], s2, (_, e) => {
    console.log("got a ", e.payload.part);
    return { part: e.payload.part } })

s2.react([Events.partOK], s0, (_, e) => { return s0.make() })

// Projection of Gwarehouse over T
const projectionInfoResult: ResultData<ProjectionAndSucceedingMap> = projectionAndInformation(interfacing_swarms, subs, "T")
if (projectionInfoResult.type == 'ERROR') throw new Error('error getting projection')
const projectionInfo = projectionInfoResult.data

const checkProjResult = checkComposedProjection(interfacing_swarms, subs, "T", transporter.createJSONForAnalysis(s0))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))

// Run the adapted machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('warehouse-1')
    const machine = createMachineRunnerBT(app, tags, s0, undefined, projectionInfo.branches, projectionInfo.specialEventTypes)
    //const machine = createMachineRunner(app, tags, s0, undefined)

    for await (const state of machine) {
      console.log("transporter. state is:", state.type)
      if (state.payload !== undefined) {
        console.log("state payload is:", state.payload)
      }
      console.log()
      const s = state.cast()
      for (var c in s.commands()) {
          if (c === 'request') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1 || {}).includes('request')) {
                    s1.request()
                }
            }, getRandomInt(500, 5000))
            break
          }
          if (c === 'deliver') {
            setTimeout(() => {
                var s1 = machine.get()?.cast()?.commands() as any
                if (Object.keys(s1 || {}).includes('deliver')) {
                    s1.deliver()
                }
            }, getRandomInt(500, 8000))
            break
          }
      }
    }
    app.dispose()
}

main()