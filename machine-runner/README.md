# Machine Runner

This library offers a TypeScript DSL for writing state machines and executing them in a fully decentralised fashion using the [Actyx](https://developer.actyx.com/) peer-to-peer event stream database.
For an overview of the project this library is part of please refer to [the GitHub repository](https://github.com/Actyx/machines).

The detailed documentation of this library is provided in its JsDoc comments.

## Example usage

[More detailed tutorial can be found here](https://github.com/lucas874/machines/tree/master/docs/swarm-workflow)

We demonstrate the usage of our decentralized state machines on an example from manufacturing automation, i.e. the factory shop floor: a warehouse requests the fleet of logistics robots to pick something up and bring it somewhere else.
Our task is to write the logic for the warehouse and for each of the robots so that the job will eventually be done.
Since there are many robots we use an auction to settle who will do it.

### Declaring the machines

First we define our set of events:

```typescript
// sent by the warehouse to get things started
export const request = MachineEvent.design('request')
    .withPayload<{ id: string; from: string; to: string }>()
// sent by each available candidate transport robot to register interest
export const bid = MachineEvent.design('bid')
    .withPayload<{ robot: string; delay: number, id: string }>()
// sent by the transport robots
export const selected = MachineEvent.design('selected')
    .withPayload<{ winner: string, id: string }>()
// sent by the transport robot performing the delivery
export const deliver = MachineEvent.design('deliver')
    .withPayload<{ id: string }>()
// sent by the warehouse to acknowledge delivery
export const ack = MachineEvent.design('acknowledge')
    .withPayload<{ id: string }>()

// declare a precisely typed tuple of all events we can now choose from
export const allEvents = [request, bid, selected, deliver, ack] as const
```

Then we can declare a swarm protocol using these events:

```typescript
export const TransportOrder = SwarmProtocol.make('warehouse-factory', Events.allEvents)
```

Now we build two machines that participate in this protocol: the `warehouse` will request the material transport and acknowledge the delivery once it has taken place,
while the fleet of `robot` perform the material transport.

```typescript
// initialize the state machine builder for the `warehouse` role
export const Warehouse =
  TransportOrder.makeMachine('warehouse')

// add initial state with command to request the transport
export const InitialWarehouse = Warehouse
  .designEmpty('Initial')
  .command('request', [Events.request], (_ctx, id: string, from: string, to: string) => [{ id, from, to }])
  .finish()

// add state entered after performing the request
export const AuctionWarehouse = Warehouse
  .designEmpty('AuctionWarehouse')
  .finish()

// add state entered after a transport robot has been selected
export const SelectedWarehouse = Warehouse
  .designEmpty('SelectedWarehouse')
  .finish()

// add state for acknowledging a delivery entered after a robot has performed the delivery
export const AcknowledgeWarehouse = Warehouse
  .designState('Acknowledge')
  .withPayload<{id: string}>()
  .command('acknowledge', [Events.ack], (ctx) => [{ id: ctx.self.id }])
  .finish()

export const DoneWarehouse = Warehouse.designEmpty('Done').finish()

// describe the transition into the `AuctionWarehouse` state after request has been made
InitialWarehouse.react([Events.request], AuctionWarehouse, (_ctx, _event) => {})
// describe the transitions from the `AuctionWarehouse` state
AuctionWarehouse.react([Events.bid], AuctionWarehouse, (_ctx, _event) => {})
AuctionWarehouse.react([Events.selected], SelectedWarehouse, (_ctx, _event) => {})
// describe the transitions from the `SelectedWarehouse` state
SelectedWarehouse.react([Events.deliver], AcknowledgeWarehouse, (_ctx, event) => AcknowledgeWarehouse.make({id: event.payload.id}))
// describe the transitions from the `AcknoweledgeWarehouse` state
AcknowledgeWarehouse.react([Events.ack], DoneWarehouse, (_ctx, _event) => {})
```

The `robot` state machine is constructed in the same way:

```typescript
export const TransportRobot = TransportOrder.makeMachine('transporRobot')

export type Score = { robot: string; delay: number }
export type AuctionPayload =
  { id: string; from: string; to: string; robot: string; scores: Score[] }

export const InitialTransport = TransportRobot.designState('Initial')
  .withPayload<{ robot: string }>()
  .finish()
export const Auction = TransportRobot.designState('Auction')
  .withPayload<AuctionPayload>()
  .command('bid', [Events.bid], (ctx, delay: number) =>
                         [{ robot: ctx.self.robot, delay, id: ctx.self.id }])
  .command('select', [Events.selected], (ctx, winner: string) => [{ winner, id: ctx.self.id}])
  .finish()
export const DoIt = TransportRobot.designState('DoIt')
  .withPayload<{ robot: string; winner: string, id: string }>()
  .command('deliver', [Events.deliver], (ctx) => [{ id: ctx.self.id }])
  .finish()
export const Done = TransportRobot.designEmpty('Done').finish()

// ingest the request from the `warehouse`
InitialTransport.react([Events.request], Auction, (ctx, r) => ({
  id: r.payload.id,
  from: r.payload.from,
  to: r.payload.to,
  robot: ctx.self.robot,
  scores: []
}))

// accumulate bids from all `robot`
Auction.react([Events.bid], Auction, (ctx, b) => {
  ctx.self.scores.push({robot: b.payload.robot, delay: b.payload.delay})
  return ctx.self
})

// end the auction when a selection has happened
Auction.react([Events.selected], DoIt, (ctx, s) =>
  ({ robot: ctx.self.robot, winner: s.payload.winner, id: ctx.self.id }))

// go to the final state
DoIt.react([Events.deliver], Done, (_ctx) => {[]})
```

### Checking the machines

<img src="https://raw.githubusercontent.com/lucas874/machines/refs/heads/update-packages/demos/warehouse-readme-demo/transport-order-protocol.svg" alt="transport order workflow" width="300" />

The transport order workflow implemented in the previous section is visualized above as a UML state diagram.
With the `@actyx/machine-check` library we can check that this workflow makes sense (i.e. it achieves eventual consensus, which is the same kind of consensus used by the bitcoin network to settle transactions), and we can also check that our state machines written down in code implement this workflow correctly.

To this end, we first need to declare the graph in JSON notation:

```typescript
export const transportOrderProtocol: SwarmProtocolType = {
  initial: 'initial',
  transitions: [
    {source: 'initial', target: 'auction',
      label: {cmd: 'request', role: 'warehouse', logType: [Events.request.type]}},
    {source: 'auction', target: 'auction',
      label: {cmd: 'bid', role: 'transportRobot', logType: [Events.bid.type]}},
    {source: 'auction', target: 'delivery',
      label: {cmd: 'select', role: 'transportRobot', logType: [Events.selected.type]}},
    {source: 'delivery', target: 'delivered',
      label: {cmd: 'deliver', role: 'transportRobot', logType: [Events.deliver.type]}},
    {source: 'delivered', target: 'acknowledged',
      label: {cmd: 'acknowledge', role: 'warehouse', logType: [Events.ack.type]}},
  ]
}
```

The naming of states does not need to be the same as in our code, but the event type names and the commands need to match.
With this preparation, we can perform the behavioral type checking as follows:

```typescript
import { checkComposedProjection, checkComposedSwarmProtocol } from '@actyx/machine-check'

const robotJSON =
  TransportOrderForRobot.createJSONForAnalysis(Initial)
const warehouseJSON =
  TransportOrderForWarehouse.createJSONForAnalysis(InitialWarehouse)
const subscriptions = {
  robot: robotJSON.subscriptions,
  warehouse: warehouseJSON.subscriptions,
}

// these should all print `{ type: 'OK' }`, otherwise there’s a mistake in
// the code (you would normally verify this using your favorite unit
// testing framework)
console.log(
  checkComposedSwarmProtocol([transportOrderProtocol], subscriptions),
  checkComposedProjection([transportOrderProtocol], subscriptions, 'robot', robotJSON),
  checkComposedProjection([transportOrderProtocol], subscriptions, 'warehouse', warehouseJSON),
)
```

### Running the machines

`@actyx/machine-runner` relies upon [Actyx](https://developer.actyx.com) for storing/retrieving events and sending them to other nodes in the swarm.
In other words, Actyx is the middleware that allows the `warehouse` and `robot` programs on different computers to talk to each other, in a fully decentralized peer-to-peer fashion and without further coordination — for maximum resilience and availability.
Therefore, before we can run our machines we need to use the Actyx SDK to connect to the local Actyx service:

```typescript
const actyx = await Actyx.of(
  { appId: 'com.example.warehouse-factory', displayName: 'warehouse-factory', version: '1.0.0' })

const tags = transportOrder.tagWithEntityId('warehouse-factory')
const transportRobot = createMachineRunner(app, tags, InitialTransport, { robot: "robotId" })
const warehouse = createMachineRunner(app, tags, InitialWarehouse, undefined)
```

The `tags` can be thought of as the name of a [dedicated pub–sub channel](https://developer.actyx.com/docs/conceptual/tags) for this particular workflow instance.
We demonstrate how to create both a robot and the warehouse, even though you probably won’t do that on the same computer in the real world.

Getting the process started means interacting with the state machines:

```typescript
for await (const state of warehouse) {
  if (state.isLike(InitialWarehouse)) {
    await state.cast().commands()?.request(parts[Math.floor(Math.random() * parts.length)], "a", "b")
  }
  if (state.isL ike(AcknowledgeWarehouse)) {
    await state.cast().commands()?.acknowledge()
  }
}
```

The `warehouse` machine implements the [async iterator](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Iteration_protocols#the_async_iterator_and_async_iterable_protocols) JavaScript protocol, which makes it conveniently consumable using a `for await (...)` loop.
Exiting this loop, e.g. using `break` as shown below, will destroy the running machine, including cancelling the underlying Actyx event subscription for live updates.

Using the `robot` role we demonstrate a few more features of the machine runner:

```typescript
let IamWinner = false
for await (const state of transportRobot) {
if (state.isLike(Auction)) {
    const auction = state.cast()
    if (!auction.payload.scores.find((s) => s.robot === auction.payload.robot)) {

        auction.commands()?.bid(getRandomInt(1, 10))
        setTimeout(() => {
            const stateAfterTimeOut = transportRobot.get()
            if (stateAfterTimeOut?.isLike(Auction)) {
                stateAfterTimeOut?.cast().commands()?.select(bestRobot(auction.payload.scores))
            }
        }, 3000)
    }
    } else if (state.isLike(DoIt)) {
        const assigned = state.cast()
        IamWinner = assigned.payload.winner === assigned.payload.robot
        if (!IamWinner) break
        assigned.commands()?.deliver()
    }
}
```

The first one is that the accumulated state is inspected in the `state.isLike(Auction)` case to see whether this particular robot has already provided its bid for the auction.
If not, it will do so by invoking a command, which will subsequently lead to the emission of a `bid` event and consequently to a new state being emitted from the machine, so a new round through the `for await` loop — this time we’ll find our bid in the list, though.

The second part is that upon registering our bid, we also set a timer to expire after 5sec.
When that happens we synchronously check the _current_ state of the workflow (since it will have changed, and if some other robot got to this part first, the auction may already be over).
If the workflow still is in the `Auction` state, we compute the best robot bid (the logic in `bestRobot` is where _your expertise_ would go) and run the `select()` command to emit the corresponding event and end the auction.

The third feature becomes relevant once the auction has ended: we check if our robot is indeed the winner and record that in a variable `IamWinner`, i.e. in the current application in-memory state.
Then we can use this information in all following states as well.

### Change detection on for-await loop

When using a for-await loop with the machine runner, the loop iterates only if all of the following criteria are met:

- A 'caughtUp' event is emitted; It happens when the machine runner receives the latest event published in Actyx;
- An event between the current `caughtUp` and the previous one triggers a change to the machine's state; The state change is determined by comparing the name and payload between the state before and after the `caughtUp` event. The comparison uses the `deepEqual` function provided by the [fast-equal package](https://www.npmjs.com/package/fast-equals).

### The consequences of Eventual Consensus

The design goal of Actyx and the machine runner is to provide uncompromising resilience and availability, meaning that if a device is capable of computation it shall be able to make progress, independent of the network.
This implies that two devices that are not currently connected (which also includes the brief time lag introduced by ping times between them!) can make contradicting choices in a workflow.

In the example above, we deliberately didn’t use a `manager` or `referee` role to select the winner in the auction, since that decision maker would be a single-point-of-failure in the whole process.
Instead, each robot independently ensures that after at five seconds a decision will be made — even if two robots concurrently come to different conclusions and both emit a `selected` event.

Machine runner resolves this conflict by using only the `selected` event that comes first in the Actyx event sort order; in other words, Actyx arbitrarily picks a winner and the losing event is discarded.
If a robot saw itself winning, started the mission, and then discovers that its win turned out to be invalid, it will have to stop the mission and pick a new one.


### Composing swarms
Suppose that the warehouse is part of a larger factory facility and that items from the warehouse are
needed on the assembly line. Instead of specifying a large workflow that combines the transport order
workflow with a description of how delivered items are used on the assembly line, and then implementing
the resulting workfow, we can reuse the transport order protocol and the machines that implement it.

<img src="https://raw.githubusercontent.com/lucas874/machines/refs/heads/update-packages/demos/warehouse-readme-demo/assembly-protocol.svg" alt="assembly line workflow" width="300" />

The workflow above specifies how the `warehouse` role requests an item and acknowledges its delivery, and how an `assemblyRobot`
uses the delivered item to assemble a product. Unlike the transport order workflow, this workflow does not specify
exactly how requested items are obtained. It simply states that an item can be requested and sometimes later its delivery can be acknowledged.

We can, however, implement the `assemblyRobot` for for the workflow above and then **automatically adapt** this machine
and the machines from the transport order protocol to work together. The resulting machines will then implement the workflow below:

<p id="composition">
<img src="https://raw.githubusercontent.com/lucas874/machines/refs/heads/update-packages/demos/warehouse-readme-demo/composition.svg" alt="composed workflow" width="300" />
</p>

To achieve this start by defining the event emitted by the assembly robot when a product has been finished.
```typescript
// the previously defined events

...

// sent by the assembly robot when a product has been assembled
export const product = MachineEvent.design('product')
    .withPayload<{productName: string}>()

// declare a precisely typed tuple of all events we can now choose from
export const allEvents = [request, bid, selected, deliver, ack] as const
```

We then declare a new swarm protocol using these events:
```typescript
export const AssemblyLine = SwarmProtocol.make('warehouse-factory', Events.allEvents)
```

Now we build the assembly robot for this protocol. Once an item has been delivered from the warehouse, it will use it to assemble a product.
```typescript
export const AssemblyRobot = AssemblyLine.makeMachine('assemblyRobot')

export const InitialAssemblyRobot = AssemblyRobot.designEmpty('Initial')
  .finish()
export const Assemble = AssemblyRobot.designState('Assemble')
  .withPayload<{id: string}>()
  .command('assemble', [Events.product], (_ctx) =>
                         [{ productName: "product" }])
  .finish()
export const Done = AssemblyRobot.designEmpty('Done').finish()

// ingest the request from the `warehouse`
InitialAssemblyRobot.react([Events.ack], Assemble, (ctx, a) => ({
  id: a.payload.id
}))

// go to the final state
Assemble.react([Events.product], Done, (ctx, b) => {})
```

As with the transport order protocol, we can perform the behavioral type checking:

```typescript
import { checkComposedProjection, checkComposedSwarmProtocol } from '@actyx/machine-check'

export const assemblyLineProtocol: SwarmProtocolType = {
  initial: 'initial',
  transitions: [
    {source: 'initial', target: 'wait',
      label: { cmd: 'request', role: 'warehouse', logType: [Events.request.type]}},
    {source: 'wait', target: 'assemble',
      label: { cmd: 'acknowledge', role: 'warehouse', logType: [Events.ack.type]}},
    {source: 'assemble', target: 'done',
      label: { cmd: 'assemble', role: 'assemblyRobot', logType: [Events.product.type] }},
  ]
}

const assemblyRobotJSON =
  AssemblyRobot.createJSONForAnalysis(AssemblyRobotInitial)
const subscriptionsForAssemblyLine = {
  assemblyRobot: assemblyRobotJSON.subscriptions,
  warehouse: [Events.request.type, Events.ack.type],
}

// these should all print `{ type: 'OK' }`
console.log(
  checkComposedSwarmProtocol([assemblyLineProtocol], subscriptionsForAssemblyLine),
  checkComposedProjection([assemblyLineProtocol], subscriptionsForAssemblyLine, 'assemblyRobot', assemblyRobotJSON)
)
```

To make the machines implemented for the transport order protocol and the assembly robot machine work together with we must *adapt* them to the [composition
of the transport order and the assembly line protocols](#composition).

We do this by first generating a subscription that works for the composed swarm:

```typescript
// subscription for the composed swarm
const resultSubsComposition: DataResult<Subscriptions>
  = overapproxWFSubscriptions([transportOrderProtocol, assemblyLineProtocol], {}, 'TwoStep')
if (resultSubsComposition.type === 'ERROR') throw new Error(resultSubsComposition.errors.join(', '))

export const subscriptions: Subscriptions = resultSubsComposition.data
```

Finally, using the subscription, we can adapt the machines and run them using *branch-tracking* MachineRunners:

```typescript
// Adapted machines.
export const [assemblyRobotAdapted, initialAssemblyAdapted] =
  AssemblyProtocol.adaptMachine('assemblyRobot',
  [transportOrderProtocol, assemblyLineProtocol], 1,
  subscriptions, [AssemblyRobot, InitialAssemblyRobot], true).data!

export const [transportAdapted, initialTransportAdapted] =
  TransportOrder.adaptMachine('transportRobot',
  [transportOrderProtocol, assemblyLineProtocol], 0,
  subscriptions, [TransportRobot, InitialTransport], true).data!

export const [warehouseAdapted, warehouseInitialAdapted] =
  TransportOrder.adaptMachine('warehouse',
  [transportOrderProtocol, assemblyLineProtocol], 0,
  subscriptions, [Warehouse, InitialWarehouse], true).data!

...

// Branch-tracking machine runners
const assemblyRobot = createMachineRunnerBT(app, tags, initialAssemblyAdapted, undefined, assemblyRobotAdapted)
const transportRobot = createMachineRunnerBT(app, tags, initialTransportAdapted, initialPayload, transportAdapted)
const warehouse = createMachineRunnerBT(app, tags, initialWarehouseAdapted, undefined, warehouseAdapted)
```



With these preparations in place, we can construct a composed swarm where the assembly robot and the machines implemented
for the transport order work together. We do this by *adapting* the macines to the composition
of the transport order and the assembly line protocols.


The transport order workflow offers the functionality of orchestrating
a fleet of robots to deliver some requested item from the warehouse. We can reuse this functionality
in another protocol by specifying a workflow that uses the `warehouse` role to handle material transportation.




Notice how the `warehouse` role from the `transportOrderProtocol` appears in the workflow above. By doing so we can implement a

### Errors

The following section describes various unavoidable errors that can arise due to the language design of JavaScript and TypeScript combined with the library's inherent distributed and asynchronous nature.

#### Errors Emittable On Command Calls

A command call returns a promise. The promise's resolution marks the success of the events' publication to Actyx.

```typescript
const whenParticularState = state.as(ParticularState)
if (whenParticularState) {
  // awaits the promise, which consequently may break the control flow if there is a thrown exception / promise rejection
  await whenParticularState.commands()?.someCommand()
}
```

However, certain scenarios can result in async errors (i.e. a rejected promise) which will be explained momentarily.

To avoid errors, the general best practice is to not stash commands in an external variable and defer the call.
The `commands()` will return undefined when possible errors are detected.
The passing of time might have invalidated the detection result, which is the stashed value.

```typescript
// Good
await whenParticularState.commands()?.someCommand()

// Bad
const commands = whenParticularState.commands()
await someLongRunningTask() // this may have invalidated the error detection when finished executing
await commands?.someCommand()
```

Avoid accidentally not issuing commands by using state objects produced by `next`,
`peek`, and `for-await` loop, as opposed to the `get` method. These methods
ensure the newly retrieved state objects are not expired. It is recommended to
only use `get` method when it is necessary to retrieve state objects immediately
(without waiting for the machine to finish processing the next event batch) at
the cost of possibly getting expired, locked, or non-caught-up state objects,
such as when observing a machine runner's state passively.

```typescript
// Guaranteed working, assuming there's no other
const whenOn = (await r.machine.peek()).value?.as(On)
if (whenOn) {
  await whenOn.commands()?.toggle() // returns promise
}

// There's a chance that commands is not available
const whenOn = r.machine.get().value?.as(On)
if (whenOn) {
  await whenOn.commands()?.toggle() // may be undefined
}
```

The list of errors that may arise is as follows.

- `MachineRunnerErrorCommandFiredAfterExpired`:

  This error results from a command call to an expired state or a machine-runner with a non-empty event queue.

  An "Expired" state object is a state object that does not match its machine-runner's current state object. An expired state object can be obtained by means of storing a state object's reference in a variable and using it at a later time when the machine-runner has transitioned to another state.

  The non-empty event queue condition happens when a machine-runner is waiting for the completion of a multi-events transition. In a multi-events transition, different parts of this ordered events chain can arrive at different points in time. The queue is not empty in the period between the first and the last arrival.

- `MachineRunnerErrorCommandFiredAfterLocked`:

  This error results from a command call to a locked state object/machine-runner. It is generally avoidable by not having a command being called twice in a concurrent fashion in the same state.

  "Locked" is a transitionary state object/machine-runner between the time a command is called and it receives a rejection. When a command results in a failed publication, the source state objects are unlocked, thus subsequent commands will be available. When a command results in a successful publication, the machine-runner is unlocked, but the issuing state object is kept locked to prevent subsequent command calls. The machine-runner will immediately produce a new state object with commands enabled, accessible via `next`, `peek`, and `for-await` loop.

- `MachineRunnerErrorCommandFiredAfterDestroyed`:

  This error results from a command call to a state object of a destroyed machine.

  "Destroyed" is the status of a machine-runner that has been destroyed, either by explicitly calling its `.destroy()` method or by breaking out of the `for-await` that is applied to the runner.

- `MachineRunnerErrorCommandFiredWhenNotCaughtUp`:

  This error results from a command call to a state object whose machine is not caught up.

  "Caught up" is a state where a machine-runner has processed all published events in a subscription stream. Actyx sends events in batches. During the processing of a batch, the machine-runner is not caught up.

## Features

### Observe Changes and Errors

An alternative use case of a machine runner is to listen to its events.

The `next` event emits states whenever a new state is calculated.
When not using the machine, calling `destroy` is required to close the connection to Actyx.

```typescript
const warehouse = createMachineRunner(actyx, tags, InitialWarehouse, { id: '4711' })

warehouse.events.on('next', (state) => {
  if (state.is(InitialWarehouse)) {
    // ...
  }
})

await untilWareHouseIsNotUsedAnymore()

warehouse.destroy()
```

`error` event can be used to capture errors from machine-runner.

```
import {
  MachineRunnerErrorCommandFiredAfterLocked,
  MachineRunnerErrorCommandFiredAfterDestroyed,
  MachineRunnerErrorCommandFiredAfterExpired,
} from "@actyx/machine-runner"

warehouse.events.on('error', (error) => {
  if (error instanceof MachineRunnerErrorCommandFiredAfterLocked) {
    //
  }

  if (error instanceof MachineRunnerErrorCommandFiredAfterDestroyed) {
    //
  }

  if (error instanceof MachineRunnerErrorCommandFiredAfterExpired) {
    //
  }
})
```

#### Event List

##### `next`

A `next` event is emitted when a state transition happens and the machine runner has processed all events matching the supplied tag.

The payload is `StateOpaque`, similar to the value produced in the `for-await` loop.

##### `error`

An `error` event is emitted when an error happened inside the runner. Currently this is the list of the errors:

- A command is called when locked i.e. another command is being issued in the same machine
- A command is called when the corresponding state is expired i.e. another command has been successfully issued from that state
- A command is called on a destroyed machine

The payload has an error subtype.

##### `change`

A `change` event is emitted when a `next` event is emitted, a command is issued, a command’s event has been published, or a subscription error happened due to losing a connection to Actyx.
This event is particularly useful in UI code where not only state changes are tracked, but also command availability and errors.

The payload is of type `StateOpaque`, like the value produced in the `for-await` loop.

##### `debug.bootTime`

A `debug.bootTime` event is emitted when a machine runner has caught up with its Actyx subscription (i.e. finished processing its events until the latest one) for the first time.

The payload includes information on the duration of the booting, the number of events processed, and the identity containing the swarm name, machine name, and tags.

```typescript
// Logs every time a machine booting takes more than 100 milliseconds or processed more than 100 events
machine.events.on(
  'debug.bootTime',
  ({ durationMs, eventCount, identity: { machineName, swarmProtocolName, tags } }) => {
    if (durationMs > 100 || eventCount > 100) {
      console.warn(
        `Boot of "${swarmProtocolName}-${machineName}" tagged "${tags.toString()}" takes longer than usual (${durationMs} milliseconds of to process ${eventCount} events)`,
      )
    }
  },
)
```

### Zod on MachineEvent

[Zod](https://zod.dev/) can be used to define and validate MachineEvents. On designing an event, use `withZod` instead of `withPayload`.

```typescript
export const requested = MachineEvent.design('requested').withPayload<{
  id: string
  from: string
  to: string
}>()
```

The above code can be converted into:

```typescript
import * as z from 'zod'

export const requested = MachineEvent.design('requested').withZod(
  z.object({
    id: z.string(),
    from: z.string(),
    to: z.string(),
  }),
)
```

A zod-enabled event factory will have these additional features enabled:

- When receiving events from Actyx, a `MachineRunner` will compare the event payload to the embedded `ZodType`, in addition to the mandatory event type checking. Events that don't match the defined `MachineEvent` on the reaction will be ignored by the `MachineRunner`. For example, see the reaction definition below:
  ```typescript
  InitialWarehouse.react([requested], DoneWarehouse, (_ctx, _r) => [{}])
  ```
  In a scenario where an incorrectly created event comes from Actyx `{ "type": "requested", id: "some_id" }`, the said event will not be regarded as valid and will be ignored.
- When creating an event via the factory, which would be `requested.make` for the example above, an extra step will be taken to validate the payload. When the `make` method is called with an incorrect value, an exception will be thrown because internally `ZodType.parse` is used to validate the payload. For example:

  ```typescript
  // Will throw because `{}` is not a valid value for the previously provided zod schema
  // But it takes `as any` to bypass TypeScript compiler in order to do this
  const singleEvent = requested.make({} as any)
  ```

  An extra care must be taken when the `ZodType` is [refined](https://zod.dev/?id=refine). In contrast to a mismatch in schema, a refined `ZodType` is not caught at compile-time. Therefore, a compilation process and IDE warnings is not sufficient to catch these errors. For example:

  ```typescript
  export const requested = MachineEvent.design('requested').withZod(
    z
      .object({
        id: z.string(),
        from: z.string(),
        to: z.string(),
      })
      .refine((payload) => {
        return payload.from == payload.to
      }),
  )

  // Will throw exception because `from` is same with `to`.
  // This mistake can't be caught by TypeScript compiler
  requested.make({
    id: 'some_id',
    from: 'some_location',
    to: 'some_location',
  })
  ```

### Global Event Emitter

Some global event emitters are provided.
These event emitters will emit events from all machine runners in the same process.

```typescript
import { globals as machineRunnerGlobals } from "@actyx/machine-runner";

globals.emitter.addListener("debug.bootTime", ({ identity, durationMs, eventCount }) => {
  if (durationMs > 100) {
    console.warn(`${identity} boot time takes more than 100ms (${durationMs}ms) processing ${eventCount} events`);
  }
});

globals.emitter.addListener("error", console.error);
```

### Extra Tags

In the case extra tags are required to be attached in events when invoking commands, extra tags can be registered on a command definition. These extra tags will always be attached when the command is invoked.

```typescript
// State definition
export const InitialWarehouse = TransportOrderForWarehouse.designState('Initial')
  .withPayload<{ id: string }>()
  .command('request', [requested], (ctx, from: string, to: string) => {
    return [
      ctx.withTags(
        [`transport-order-from:${from}`, `transport-order-to:${to}`],
        { id: ctx.self.id, from, to }
      )
    ]
  })
  .finish()

// Command call
// The resulting events will include the extra tags
// `transport-order-from:${from}`,
// `transport-order-to:${to}`
const stateAsInitialWarehouse = state
  .as(InitialWarehouse)?
  .commands()?
  .request(from: `source`, to: `destination`);
```

### `refineStateType`

A `MachineRunner` instance now has a new method available: `refineStateType` which return a new aliasing machine.
State payload produced by the returned machine is typed as the **union of all possible payload types** instead of `unknown`.
The union is useful to be used in combination with [type-narrowing](https://www.typescriptlang.org/docs/handbook/2/narrowing.html).

Usage example:

```typescript
// States defined in previous examples
const allStates = [Initial, Auction, DoIt] as const
const machine = createMachineRunner(actyx, tags, Initial, { robot: 'agv1' }).refineStateType(
  allStates,
)

const state = machine.get()
if (state) {
  const payload = state.payload

  // Equals to:
  //  | { robot: string }
  //  | { id: string; from: string; to: string; robot: string; scores: Score[] }
  //  | { robot: string; winner: string }
  type PayloadType = typeof state.payload

  // 'robot' property is accessible directly because it is available in all variants
  const robot = payload.robot

  // Used with type-narrowing
  if ('winner' in payload) {
    // here the type of payload is narrowed to { robot: string; winner: string }
  } else if ('id' in payload) {
    // here the type of payload is narrowed to { id: string; from: string; to: string; robot: string; scores: Score[] }
  } else {
    // here the type of payload is narrowed to { robot: string }
  }
}
```

The argument to `.refineStateType` must be an array containing all previously defined states.
Any other argument will throw an error.

The aliasing machine shares the original machine's internal state.
All method calls, such as `.destroy`, create the same effect as when enacted on the original machine.

## Developer support

If you have any questions, suggestions, or just want to chat with other interested folks, you’re welcome to join our discord chat. Please find a current invitation link on [the top right of the Actyx docs page](https://developer.actyx.com/).
