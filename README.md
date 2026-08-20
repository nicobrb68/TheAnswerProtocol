## Commande pour test le flood

```bash
(echo "CONNECT flooder"; for i in $(seq 1 25); do echo "WHO"; done; echo "QUIT") |\
 nc localhost 7534
```