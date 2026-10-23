# Spec Delta

## ADDED Requirements

### Requirement: The first list holds past conversations
The system SHALL answer the first list request after the host starts only once its first import of the agents' history has finished, waiting at most 10 seconds.

#### Scenario: Opened right after the host starts
- **WHEN** a client lists sessions while the host's first history import is still running
- **THEN** the answer comes once the import has finished and includes the imported conversations
