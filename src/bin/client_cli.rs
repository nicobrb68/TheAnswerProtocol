use std::borrow::Cow;
use tokio::net::TcpStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, CmdKind};
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Editor, Helper};

struct TapCompleter {
    commands: Vec<String>,
}

impl Helper for TapCompleter {}
impl Hinter for TapCompleter { type Hint = String; }
impl Highlighter for TapCompleter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if line.is_empty() {
            return Cow::Borrowed(line);
        }

        let parts: Vec<&str> = line.splitn(3, ' ').collect();
        let first = parts[0].to_uppercase();

        let single = ["CONNECT", "LOOK", "MOVE", "WHO", "TAKE", "DROP",
            "INVENTORY", "TALK", "ATTACK", "STATUS", "QUEST", "QUESTS", "QUIT"];
        let chat_subs = ["GLOBAL", "ROOM", "GROUP"];
        let group_subs = ["CREATE", "INVITE", "JOIN", "LEAVE", "DISBAND", "INFO", "KICK"];

        let (valid, cmd_end) = 
        if first == "CHAT" || first == "GROUP" {
            if let Some(sub) = parts.get(1) {
                let sub_upper = sub.to_uppercase();
                let subs = if first == "CHAT" { &chat_subs[..] } else { &group_subs[..] };
                if subs.contains(&sub_upper.as_str()) {
                    (true, parts[0].len() + 1 + sub.len())
                } else {
                    (true, parts[0].len())
                }
            } else {
                (true, parts[0].len())
            }
        } else if single.contains(&first.as_str()) {
            (true, parts[0].len())
        } else {
            (false, parts[0].len())
        };

        let cmd = &line[..cmd_end];
        let rest = &line[cmd_end..];

        if valid {
            Cow::Owned(format!("\x1b[32m{}\x1b[0m\x1b[36m{}\x1b[0m", cmd, rest))
        } else {
            Cow::Owned(format!("\x1b[31m{}\x1b[0m{}", cmd, rest))
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: CmdKind) -> bool {
        true
    }
}
impl Validator for TapCompleter {}

impl Completer for TapCompleter {
    type Candidate = Pair;
    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let input = &line[..pos].to_uppercase();
        let matches: Vec<Pair> = self
            .commands
            .iter()
            .filter(|cmd| cmd.starts_with(input))
            .map(|cmd| Pair {
                display: cmd.clone(),
                replacement: cmd.clone(),
            })
            .collect();
        Ok((0, matches))
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <host> <port>", args[0]);
        std::process::exit(1);
    }
    let addr = format!("{}:{}", args[1], args[2]);

    let stream = match TcpStream::connect(&addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to connect to {}: {}", addr, e);
            std::process::exit(1);
        }
    };
    eprintln!("Connected to {}", addr);

    let (reader, writer) = stream.into_split();
    let (input_tx, mut input_rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // Thread rustyline — lit l'input avec autocompletion
    std::thread::spawn(move || {
        let completer = TapCompleter {
            commands: vec![
                "CONNECT".into(), "LOOK".into(), "MOVE".into(),
                "CHAT GLOBAL".into(), "CHAT ROOM".into(), "CHAT GROUP".into(),
                "WHO".into(), "TAKE".into(), "DROP".into(), "INVENTORY".into(),
                "TALK".into(), "ATTACK".into(), "STATUS".into(),
                "QUEST".into(), "QUESTS".into(),
                "GROUP CREATE".into(), "GROUP INVITE".into(), "GROUP KICK".into(),
                "GROUP JOIN".into(), "GROUP LEAVE".into(), "GROUP DISBAND".into(),
                "GROUP INFO".into(),
                "QUIT".into()
            ],
        };
        let mut rl = Editor::new().expect("Failed to create editor");
        rl.set_helper(Some(completer));
        loop {
            match rl.readline("> ") {
                Ok(line) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    rl.add_history_entry(&line).ok();
                    if input_tx.send(format!("{}\n", line)).is_err() {
                        break;
                    }
                    if line.trim().eq_ignore_ascii_case("QUIT") {
                        break;
                    }
                }
                Err(ReadlineError::Eof | ReadlineError::Interrupted) => break,
                Err(_) => break,
            }
        }
    });

    // Task recv : serveur → terminal
    let mut server_reader = BufReader::new(reader);
    let task_recv = tokio::spawn(async move {
        let mut line = String::new();
        loop {
            line.clear();
            match server_reader.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    // Efface la ligne "> " courante, affiche le message, reaffiche "> "
                    eprint!("\r\x1b[2K");
                    print!("{}", line);
                    eprint!("> ");
                }
                Err(_) => break,
            }
        }
        eprintln!("\r\x1b[2KServer disconnected.");
    });

    // Task send : channel rustyline → serveur
    let mut writer = writer;
    let task_send = tokio::spawn(async move {
        while let Some(line) = input_rx.recv().await {
            if writer.write_all(line.as_bytes()).await.is_err() {
                break;
            }
        }
    });

    tokio::select! {
        _ = task_recv => {},
        _ = task_send => {},
    }
}
