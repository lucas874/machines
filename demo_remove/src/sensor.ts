import { Actyx } from '@actyx/sdk'
import { createMachineRunner } from '@actyx/machine-runner'
import { Events, manifest, protocol } from './protocol'

const machine = protocol.makeMachine('sensor')
export const s0 = machine.designEmpty('Thirsty')
    .command('req', [Events.NeedsWater], () => { console.log("hej"); return [Events.NeedsWater.make({requiredWaterMl: 5})] })
    .command('done', [Events.Done], () => [{}])
    .finish()
export const s1 = machine.designEmpty('Wet')
    .command('get', [Events.HasWater], () => [{}])
    .finish()
export const s2 = machine.designEmpty('isDone').finish()

s0.react([Events.NeedsWater], s1, (_) => s1.make())
s0.react([Events.Done], s2, (_) => s2.make())
s1.react([Events.HasWater], s0, (_) => s0.make())

var m = machine.createJSONForAnalysis(s0)
//console.log(m)
const [m2, i2] = protocol.makeProjMachine("sensor", m, Events.All)
const cMap = new Map()
cMap.set(Events.NeedsWater.type, () => { console.log("hej"); return [Events.NeedsWater.make({requiredWaterMl: 5})]})
cMap.set(Events.Done.type, () => [{}])
cMap.set(Events.HasWater.type, () => [{}])

const rMap = new Map()
rMap.set(Events.NeedsWater, () => [Events.NeedsWater.make({requiredWaterMl: 5})])
rMap.set(Events.Done, () => [{}])
const statePayloadMap = new Map()
const fMap : any = {commands: cMap, reactions: rMap, statePayloads: statePayloadMap}

//const [m3, i3] = protocol.extendMachine("sensor", m, Events.All, [machine, s0], fMap)
const [m3, i3] = protocol.extendMachine("sensor", m, Events.All, fMap)

//const _ = protocol.extendMachine("sensor", m, Events.All, [machine, s0])

async function main() {
  const sdk = await Actyx.of(manifest)
  const tags = protocol.tagWithEntityId('robot-1')
  const machine = createMachineRunner(sdk, tags, i3, undefined)
  var hasRequested = false
  var isDone = false
  for await (const state of machine) {
    console.log("state is: ", state)
    if (isDone) {
        console.log("shutting down")
        break
    }

    const t = state.cast()
    //console.log("t: ", t)
    //console.log("to.commands()?", t.commands())
    //console.log(state.commandsAvailable())
    for (var c in t.commands()) {
        var tt = t.commands() as any;
        if (c === 'req' && !hasRequested) {
            //console.log("found: ", c)
            setTimeout(() => {
                //console.log("has req: ", hasRequested)
                if (!hasRequested) {
                    hasRequested = true
                    tt?.req()
                }
            }, 3000)
            break
        } else if (c === 'get') {
            setTimeout(() => {
                tt?.get()
            }, 3000)
            break
        } else if (c === 'done') {
            tt?.done()
            isDone = true
            break
        }
    }

  }
  sdk.dispose()
}

main()


/*

if (state.is(s0)) {
        const open = state.cast()
        setTimeout(() => {
            if (!hasRequested) {
                hasRequested = true
                open.commands()?.req()
            } else {
                open.commands()?.done()
            }
        }, 3000)
    } else if (state.is(s1)) {
        const open = state.cast()
        setTimeout(() => {
            open.commands()?.get()
        }, 3000)
    } else if (state.is(s2)) {
        console.log("shutting down")
        break
    }
*/