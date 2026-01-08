# Detach aka Taskforce mode (TBD)

Sometimes we want to allow to switch to a new system locally. This mean ignoring the version that is provided by the server and instead using a locally supplied one.

This process is called Detaching. Once a client decides to start using the server supplied version again the client gets re-Attached. Detaching is usefull in multiple different scenarios:

- Testing a configuration before rolling configuration
- Installing time critical services (e.g. if you are in an organization where modifying the system is not the norm)

## Design

Because the daemon does by design not store any of its state this also includes the current state of attachment. Meaning that if the agent wants to detach he has to consult the server first.
This parts also allows for custom policy and rules to apply because the server can refuse the detachment.

Once the server allowed the detachment the server is always replying with the `AgentAction::Detach` this signals the agent to allow any detach requests.
The agent has to never store information about its state because the server will tell him. `AgentAction::Detach` only changes the actions taken by the Varlink service.
The agent part of the daemon still continues to report the current system in the specified time interval, providing the server with information about the current closure.

### Local only detach

There may be times were no connection to the yeet server is possible or event wanted. For this event the `--force` option exists, signaling the daemon to not contact the server and instead force-switch to a new derivation.
The implication is, that once the daemon regains connection to the yeet server he will automatically switch to provisioned state and use the server provided version, rolling back any changes made by the force switch.

### Server

The server should still save updates for a detached client. Once the agent attaches he should get the latest version instead of the version when he detached.
