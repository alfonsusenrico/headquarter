## ADDED Requirements

### Requirement: Resources of running sessions
The host SHALL report, when a client asks, its own CPU and memory and, for each running session, the CPU and memory of the session's agent and all its descendant processes, and of its shell panel, with the number of processes beyond the agent. CPU SHALL be given per core and memory as resident memory.

#### Scenario: A session with a child process
- **WHEN** a session's agent has started a child process that uses CPU, and a client asks twice
- **THEN** the second answer includes the child in the session's figures and counts one process beyond the agent
