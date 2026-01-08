# Varlink

Varlink allows the user facing cli to perform actions that would otherwise require configuration or authentication.
The idea is that even if the user is not an admin, if the yeet server allows it, the user can perform system modifying action by leveragin the running yeet agent. \
Furthermore if the user wants to execute commands on the yeet server, instead of the cli executing the command itself it will ask the daemon to do it (TBD)

This is different for `yeet server` commands as these are for non interactive use or for debugging.

## Security

The yeet-daemon will at startup create a socket at `/run/yeet/agent.varlink`. This socket is only read+write for the `yeet` group.
Even for basic status queries the user has to be in the `yeet` group.

TBD:
If the user whishes to execute a varlink method that requires more permissions e.g. detach the daemon, a polkit request is created to authenticate the user.
This allows administrators to fine-tune the actions a user can take on the yeet daemon.

An example configuration includes the user always in the yeet group (to allow the status query) but disallows any higher functions.

## Design

The varlink thread is not coupled with the agent thread at all. There is no communication that exists between the different thread. Meaning that all information the daemon requires, besides configuration, he has to get by himself.
This includes querying the yeet server for information about its state. \
Because the whole yeet agent does not contain any state this will not inconsistencies.
But there is still room for race conditions: If the user performs a modifying action like switching to a new system while the agent thread is currently switching systems themself this can lead to race conditions between the two threads (To be verified). A simple fix is to implement a simple Thread Lock when any modifying actions are executed (TBD).
