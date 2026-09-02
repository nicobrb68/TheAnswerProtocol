let ws = null

if (ws !== null && ws.readyState == WebSocket.OPEN) {
	window.addEventListener("beforeunload", () => {
		ws.send("QUIT")
	})
}

function connect_user(host, port, username) {
	ws = new WebSocket("ws://127.0.0.1:3000/ws")

	ws.addEventListener("open", () => {
		const connectMsg = {
			host: host,
			port: parseInt(port),
			username: username
		}

		ws.send(JSON.stringify(connectMsg));
	})

	ws.addEventListener("message", (event) => {
		console.log(event.data)
	})
}

function sendCommand(cmd) {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(cmd);
    }
}

function showLoginError(msg) {
    const el = document.querySelector("#login-error");
    el.textContent = msg;
    el.style.display = "block";
}

document.querySelector("#connect-form").addEventListener("submit", (e) => {
	e.preventDefault();
	const data = new FormData(e.target);
	const host = data.get("host");
	const port = data.get("port");
	const username = data.get("username");
	if (!host && !port && !username) {
		showLoginError("Fill in all fields");
		return
	} else if (!host) {
		showLoginError("Host is required");
		return
	} else if (!port) {
		showLoginError("Port is required");
		return
	} else if (!username) {
		showLoginError("Username is required");
		return
	}

	
	if (ws !== null && ws.readyState == WebSocket.OPEN) {
    	sendCommand("CONNECT " + username);
	} else {
		connect_user(host, port, username);
	}
})

