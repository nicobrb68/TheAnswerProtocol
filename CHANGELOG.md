# Journal de bord - TAP

## 01/08 - 1h22 du mat

Commencé le projet avec Claude comme prof de Rust.
Jamais fait de Rust de ma vie, maintenant j'ai un serveur TCP qui tourne.

Ce qu'on a fait :
- setup du projet Cargo (tokio, serde, tracing)
- serveur TCP async qui accepte plusieurs clients en parallèle
- lecture des commandes ligne par ligne
- parsing de CONNECT + réponse OK connected
- greeting proto=1 à la connexion

Prochaine étape : les structs (Room, Player, World) + Arc<Mutex<T>>
pour partager l'état entre les clients.

P.S. Claude m'apprend plutot bien et meme si cest merdiquement verbeux le rust
cest assez fun en vrai