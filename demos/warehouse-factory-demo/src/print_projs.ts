import { Actyx } from '@actyx/sdk'
import { createMachineRunner, ProjMachine } from '@actyx/machine-runner'
import { Events, manifest, Composition, warehouse_factory_protocol, subs_composition, all_projections, getRandomInt  } from './protocol'
import { projectCombineMachines } from '@actyx/machine-check'

for (var p of all_projections) {
    console.log(JSON.stringify(p))
    console.log()
    console.log("$$$$")
    console.log()
}