# Context Map (v1)

## Monitoring Context
Responsibility
- Periodically check WatchTargets
- Produce Events when changes are detected

Outputs:
- Event

## Eventlog Context
Responsibility:
- Apply Policy (cooldown)
- Deliver notifications through Notifiers

Inputs:
- Event
Outputs:
- Notification delivery results (logged)