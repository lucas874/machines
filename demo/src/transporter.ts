import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, subswh, subsf, all_projections, getRandomInt  } from './factory_protocol'
import { projectCombineMachines, checkComposedProjection } from '@actyx/machine-check'

/*

Using the machine runner DSL an implmentation of transporter in Gwarehouse is:

const transporter = Composition.makeMachine('T')
export const s0 = transporter.designState('s0').withPayload<{id: string}>()
    .command('request', [Events.partID], (s: any, e: any) => {
      var id = s.self.id;
      console.log("requesting a", id);
      return [Events.partID.make({id: id})]})
    .finish()
export const s1 = transporter.designEmpty('s1').finish()
export const s2 = transporter.designState('s2').withPayload<{part: string}>()
    .command('deliver', [Events.part], (s: any, e: any) => {
      console.log("delivering a", s.self.part)
      return [Events.part.make({part: s.self.part})] })
    .finish()
export const s3 = transporter.designEmpty('s3').finish()

s0.react([Events.partID], s1, (_) => s1.make())
s0.react([Events.time], s3, (_) => s3.make())
s1.react([Events.position], s2, (_, e) => {
    console.log("got a ", e.payload.part);
    return { part: e.payload.part } })

s2.react([Events.part], s0, (_, e) => { return s0.make({id: ""}) })
*/

// Projection of Gwarehouse || Gfactory || Gquality over D
const result_projection = projectCombineMachines(interfacing_swarms, subs, "T")
if (result_projection.type == 'ERROR') throw new Error('error getting projection')
const projection = result_projection.data

// Command map
const cMap = new Map()
cMap.set(Events.partID.type, (s: any, e: any) => {
  var id = s.self.id;
  console.log("requesting a", id);
  return [Events.partID.make({id: id})]})

cMap.set(Events.part.type, (s: any, e: any) => {
  console.log("s is: ", s)
  console.log("delivering a", s.self.part)
  return [Events.part.make({part: s.self.part})] })

// Reaction map
const rMap = new Map()
const positionReaction : ProjMachine.ReactionEntry = {
  genPayloadFun: (_, e) => {  return { part: e.payload.part } }
}
rMap.set(Events.position.type, positionReaction)

// hacky. we use the return type of this function to set the payload type of initial state and any other state enabling same commands as in initial
const initialPayloadType : ProjMachine.ReactionEntry = {
  genPayloadFun: () => { return {part: ""} }
}
const fMap : any = {commands: cMap, reactions: rMap, initialPayloadType: initialPayloadType}

// Extended machine
const [m3, i3] = Composition.extendMachine("T", projection, Events.allEvents, fMap)

const checkProjResult = checkComposedProjection(interfacing_swarms, subs, "T", m3.createJSONForAnalysis(i3))
if (checkProjResult.type == 'ERROR') throw new Error(checkProjResult.errors.join(", "))


// Run the extended machine
async function main() {
    const app = await Actyx.of(manifest)
    const tags = Composition.tagWithEntityId('factory-1')
    const parts = ['tire', 'windshield', 'chassis', 'hood', 'spoiler']
    const machine = createMachineRunner(app, tags, i3, {id: parts[Math.floor(Math.random() * parts.length)]})

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