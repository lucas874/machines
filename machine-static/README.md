# Machine Static
Libraries to support behavioural typechecking for [machine-runner](../machine-runner/) machines and composition of swarms.

* [machine-check](machine-check) is a fork of [the Actyx machine-check library](https://github.com/Actyx/machines/tree/master/machine-check) and allows you to check whether the machines you implement with [machine-runner](../machine-runner/) comply with a correct overall swarm behaviour.
* [machine-core](machine-core) contains types and utilities used by [machine-check](machine-check) and [machine-runner](../machine-runner/) as well as functionality to automatically generate *well-formed* subscriptions and adapt machines to composed swarms.
* [evalutation](evaluation) evaluates the performance of [machine-check](machine-check) and [machine-core](machine-core).

## Acknowledgements
The development of these libraries was partly funded by the Horizon Europe project 101093006 TaRDIS - [https://project-tardis.eu/](https://project-tardis.eu/).