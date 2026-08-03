# Règle par défaut appelée si tu tapes juste 'make'
all: build

# Compile tout le projet en mode release (optimisé)
build:
	cargo build --release

# Prépare les dépendances
install:
	cargo fetch

# Lance le serveur avec le fichier world.json en paramètre
run-server:
	cargo run --bin server -- world.json

# Lance le client CLI (Ligne de commande)
run-client:
	cargo run --bin client_cli

# Lance le client GUI (Graphique)
run-client-gui:
	cargo run --bin client_gui

# Vérifie le style et les erreurs de code (clippy + fmt)
lint:
	cargo clippy -- -D warnings
	cargo fmt --check

# Supprime le dossier 'target' (fichiers compilés)
clean:
	cargo clean

nc:
	nc 127.0.0.1 1234

.PHONY: all build install run-server run-client run-client-gui lint clean