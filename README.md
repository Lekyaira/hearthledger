# Hearthledger

A centralized, family/community resource management app. Managers can keep track of inventory, and members
can request inventory to be bundled for pickup.

## Deploy

`docker compose up -d --build`
| **CAUTION**
| Application is high trust: there is no built-in authentication. You must use some form of authentication
| on the reverse proxy!
| Application is not intended to be deployed in a low trust environment.
