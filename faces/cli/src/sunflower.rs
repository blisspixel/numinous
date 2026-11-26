//! Stateless terminal play and board replay for the Party table.

use clap::Args;

#[derive(Args, Default)]
pub(super) struct Sunflower {
    /// Start from the four-digit board code shown in any face.
    #[arg(long, value_name = "CODE")]
    board: Option<String>,
    /// Draw or erase a string, e.g. --toggle 1-2. Repeat to play more moves.
    #[arg(long, value_name = "A-B", value_parser = edge)]
    toggle: Vec<[u8; 2]>,
    /// Compare three strings on the resulting board, e.g. 1-2,1-3,2-3.
    #[arg(long, value_name = "A-B,C-D,E-F", value_parser = triple)]
    inspect: Option<[[u8; 2]; 3]>,
    /// Try to keep as many strings as possible without making a sunflower.
    #[arg(long)]
    challenge: bool,
    /// Open the finite challenge's solution.
    #[arg(long)]
    reveal: bool,
    /// Emit the structured board, witness and selected intersections.
    #[arg(long)]
    json: bool,
}

fn edge(value: &str) -> Result<[u8; 2], &'static str> {
    let message = "Write two guest numbers joined by a hyphen, e.g. 1-2.";
    let (a, b) = value.split_once('-').ok_or(message)?;
    Ok([
        a.parse().map_err(|_| message)?,
        b.parse().map_err(|_| message)?,
    ])
}

fn triple(value: &str) -> Result<[[u8; 2]; 3], &'static str> {
    let mut edges = value.split(',');
    let message = "Choose exactly three strings, e.g. 1-2,1-3,2-3.";
    let selection = [
        edge(edges.next().ok_or(message)?)?,
        edge(edges.next().ok_or(message)?)?,
        edge(edges.next().ok_or(message)?)?,
    ];
    if edges.next().is_some() {
        return Err(message);
    }
    Ok(selection)
}

impl Sunflower {
    pub(super) fn report(&self) -> Result<String, String> {
        let (text, data) = crate::sunflower_report::report(
            self.board.as_deref(),
            &self.toggle,
            self.inspect,
            self.challenge,
            self.reveal,
        )?;
        if self.json {
            Ok(format!("{}\n", crate::to_pretty(&data)))
        } else {
            Ok(format!(
                "{text}\nPlay on with numinous sunflower --board {} --toggle 1-2 (change the pair to choose your string).\n",
                data["board"].as_str().expect("board code")
            ))
        }
    }
}
