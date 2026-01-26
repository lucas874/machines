# Machine Static
Libraries to support behavioural typechecking for @actyx/machine-runner machines and composition of swarms.

The library [machine-check](machine-check) allows you to check whether the machines you implement with [machine-runner](https://www.npmjs.com/package/@actyx/machine-runner) comply with a correct overall swarm behaviour.
The library [machine-core](machine-core) contains types and utilities used by [machine-check](machine-check) and [machine-runner](../machine-runner/) as well as functions for *automatically* generating well-formed subscriptions and adapting machines to composed swarms. The package found in [evalutation](evaluation) evaluates the performance of [machine-check](machine-check) and [machine-core](machine-core).