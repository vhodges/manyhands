---
id: 01M7ZBC57ZWK73V9Q9RQTNS76E
title: Implementation Queues
manyhands_managed: true
manyhands_kind: document
---
# Implementation Queues

These form the foundation for a set of features to implment the autonomous software factory pattern (basically a fancy RalphLoop).

There is an existing script (@docs/scripts/ticket_loop) that does this in a rudimentary way today for Manyhands but I feel it should be a first class feature of the software.

## Basic Traits

- Multiple queues
- Multiple agents
- Queues are named
- Agents are named
- Agents have a ticket query/filter
- Multiple implementers and other kinds of agents (reviewers, etc) 
- queues and agents decoupled from queues mostly.
- Tickets are assigned to a queue (a new field on a ticket)
- Implementers have a query/filter for tickets by default for tickets for their queue. Note: An implementer might have a different query/filter eg ticket status == review_ready to trigger code reviews, or ticket_status == merge_approved to trigger a ticket close (poor mans merge queue with HITL approvals).
- Queues and agents are per repo but operate across repos:
    - Agents with the same name across repos are merged,  
        - if they are two different query/filters
            - treate them as separate add unique
            - appending (or prepending) an identifier to the name (repo name?)
    - Queue names are similarly merged, it needs to keep track of which repos are using which queues so that when querying for tickets it can work across all repos with that queue name.
    - Agents and Queuers are managed by manyhands and commited in git.
    - This only needs to be supported (for now at least) by the cli and only on Linux (MacOS support if it's 'free')

## How I see it working in a few steps

```
manyhands-cli queue add --name  <name> --query <query> 
# Adds a named queue that will filter tickets by query
```
...
```
# Sets up the execution envs
# one per queue
manyhands-cli queue bootstrap 
```

```
#Run the dispatcher
manyhands-cli queue dispatch
```

```
#Run the agent loop for a single agent
#Run by the dispatcher
manyhands-cli queue run agent --name <agentname>
```

We'll probably need to 'auto import' agent and queue settings when:

- Adding a new repo
- Pulling down changes from one
- etc

In other words, add creates the settings files that get checked in (and adds them to the db) but you wouldn't use add to add them to the db (maybe an import command, but I have a strong preference to do it during a poll-once)

## Agents

Agents are 1 to many to repos

## Queues

Queues are 1 to many to repos

## The Excution Environment

## The dispatcher

## The agent loop

Rewrite the shell script as Rust code.  The issued prompt should come from configuration of the agent.

