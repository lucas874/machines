import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine } from '@actyx/machine-runner'
import { Events, manifest, Composition, interfacing_swarms, subs, subswh, subsf, all_projections, getRandomInt  } from './factory_protocol'
import { projectCombineMachines } from '@actyx/machine-check'

for (var p of all_projections) {
    console.log(JSON.stringify(p))
    console.log()
    console.log("$$$$")
    console.log()
}