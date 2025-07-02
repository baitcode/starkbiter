# Software Architecture 
Starkbiter is broken into a number of crates that provide different levels of abstraction for interacting with the Ethereum Virtual Machine (EVM) sandbox.

## Starkbiter Core
The `starkbiter-core` crate is the core of the Starkbiter.
It contains the `Environment` struct which acts as an EVM sandbox and the `RevmMiddleware` which gives a convenient interface for interacting with contracts deployed into the `Environment`.
Direct usage of `starkbiter-core` will be minimized as much as possible as it is intended for developers to mostly pull from the `starkbiter-engine` crate in the future. 
This crate provides the interface for agents to interact with an in memory evm. 

## Starkbiter Engine
The `starkbiter-engine` crate is the main interface for running simulations.
It is built on top of `starkbiter-core` and provides a more ergonomic interface for designing agents and running them in simulations.

## Starkbiter CLI (under construction)
The Starkbiter CLI is a minimal interface for managing your Starkbiter projects.
It is built on top of Foundry and aims to provide a similar CLI interface of setting up and interacting with Starkbiter projects.